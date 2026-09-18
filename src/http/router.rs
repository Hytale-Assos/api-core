use axum::routing::{delete, get, post};
use axum::{Router, middleware};
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::domains::{modules, servers, sessions, whitelists};
use crate::http::health;
use crate::http::middleware as auth;
use crate::ingest::webhook;
use crate::state::AppState;

/// Construit le routeur principal de l'API.
///
/// A single authentication model: every business endpoint requires a module
/// (API key + HMAC for writes) and a scope. The backoffice is a module holding
/// the `admin` scope. Only `/health` stays public.
pub fn build(state: AppState) -> Router {
    let public_routes = Router::new().route("/health", get(health::health));

    // Module registry (backoffice via `modules:*` scope).
    let module_registry = Router::new()
        .route(
            "/api/v1/modules",
            post(modules::handler::create).get(modules::handler::list),
        )
        .route("/api/v1/modules/{id}", get(modules::handler::get_one));

    // Hytale servers (global read, writes according to ownership/admin).
    let server_routes = Router::new()
        .route(
            "/api/v1/servers",
            post(servers::handler::create).get(servers::handler::list),
        )
        .route(
            "/api/v1/servers/{id}",
            get(servers::handler::get_one).delete(servers::handler::delete),
        );

    // Player sessions (per-module reads, plus player-centric views).
    let session_routes = Router::new()
        .route("/api/v1/sessions", get(sessions::handler::list))
        .route(
            "/api/v1/players/{hytale_id}/sessions",
            get(sessions::handler::list_for_player),
        );

    // Whitelist (per-module read/write, plus player-centric view).
    let whitelist_routes = Router::new()
        .route(
            "/api/v1/whitelists",
            get(whitelists::handler::list).post(whitelists::handler::create),
        )
        .route(
            "/api/v1/whitelists/{id}",
            delete(whitelists::handler::delete),
        )
        .route(
            "/api/v1/players/{hytale_id}/whitelists",
            get(whitelists::handler::list_for_player),
        );

    // Event ingestion.
    let webhook_routes = Router::new().route("/api/v1/webhook", post(webhook::receive));

    let protected = Router::new()
        .merge(module_registry)
        .merge(server_routes)
        .merge(session_routes)
        .merge(whitelist_routes)
        .merge(webhook_routes)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::module_auth,
        ));

    Router::new()
        .merge(public_routes)
        .merge(protected)
        .layer(RequestBodyLimitLayer::new(state.config.max_body_bytes))
        .layer(cors_layer(&state))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn cors_layer(state: &AppState) -> CorsLayer {
    if state.config.cors_allowed_origins.iter().any(|o| o == "*") {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let origins: Vec<_> = state
            .config
            .cors_allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(Any)
            .allow_headers(Any)
    }
}
