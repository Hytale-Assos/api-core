use axum::routing::{delete, get, post};
use axum::{middleware, Router};
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::auth;
use crate::routes;
use crate::state::AppState;

/// Construit le routeur principal de l'API.
pub fn build(state: AppState) -> Router {
    let public_routes = Router::new().route("/health", get(routes::health::health));

    // Administration (token statique `ADMIN_TOKEN`).
    let admin_routes = Router::new()
        .route(
            "/api/v1/admin/modules",
            post(routes::modules::create).get(routes::modules::list),
        )
        .route("/api/v1/admin/modules/{id}", get(routes::modules::get_one))
        .route(
            "/api/v1/admin/servers",
            post(routes::servers::create).get(routes::servers::list),
        )
        .route(
            "/api/v1/admin/servers/{id}",
            get(routes::servers::get_one).delete(routes::servers::delete),
        )
        .route(
            "/api/v1/admin/whitelists",
            post(routes::whitelists::admin_create),
        )
        .route(
            "/api/v1/admin/whitelists/{id}",
            delete(routes::whitelists::admin_delete),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::admin_auth,
        ));

    // Lecture/écriture authentifiée par module (API key + HMAC + scopes).
    let module_routes = Router::new()
        .route("/api/v1/sessions", get(routes::sessions::list))
        .route(
            "/api/v1/whitelists",
            get(routes::whitelists::list).post(routes::whitelists::create),
        )
        .route("/api/v1/whitelists/{id}", delete(routes::whitelists::delete))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::module_auth,
        ));

    // Ingestion d'événements (API key + HMAC + scopes).
    let webhook_routes = Router::new()
        .route("/api/v1/webhook", post(routes::webhook::receive))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::module_auth,
        ));

    Router::new()
        .merge(public_routes)
        .merge(admin_routes)
        .merge(module_routes)
        .merge(webhook_routes)
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
