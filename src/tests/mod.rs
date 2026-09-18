//! Tests d'intégration HTTP : montent le routeur réel et vérifient
//! l'authentification, les scopes, le périmètre et les statuts de bout en bout.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use crate::http::router;
use crate::shared::scopes;
use crate::testutil::{register_module, sign, test_state};

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn router_for(pool: &PgPool) -> Router {
    router::build(test_state(pool.clone()))
}

fn json_request(method: &str, uri: &str, body: Option<serde_json::Value>) -> Request<Body> {
    let builder = Request::builder().method(method).uri(uri);
    match body {
        Some(value) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&value).unwrap()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

/// Requête `GET` authentifiée par API key.
fn get_with_key(uri: &str, api_key: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header("x-api-key", api_key)
        .body(Body::empty())
        .unwrap()
}

/// Requête signée HMAC (POST/DELETE).
fn signed_request(
    method: &str,
    uri: &str,
    api_key: &str,
    hmac_secret: &str,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let payload = body
        .map(|v| serde_json::to_vec(&v).unwrap())
        .unwrap_or_default();
    let timestamp = now().to_string();
    let signature = sign(hmac_secret, &timestamp, &payload);

    Request::builder()
        .method(method)
        .uri(uri)
        .header("x-api-key", api_key)
        .header("x-timestamp", timestamp)
        .header("x-signature", signature)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload))
        .unwrap()
}

/// Envoie une requête et renvoie statut + corps JSON.
async fn send(app: Router, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, value)
}

#[sqlx::test]
async fn health_is_public(pool: PgPool) {
    let (status, body) = send(router_for(&pool), json_request("GET", "/health", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[sqlx::test]
async fn protected_route_without_key_is_unauthorized(pool: PgPool) {
    let (status, _) = send(
        router_for(&pool),
        json_request("GET", "/api/v1/servers", None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn unknown_api_key_is_unauthorized(pool: PgPool) {
    let (status, _) = send(
        router_for(&pool),
        get_with_key("/api/v1/servers", "cle-inconnue"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn get_with_key_requires_no_hmac(pool: PgPool) {
    let creds = register_module(&pool, "reader", &[scopes::HYTALE_SERVERS_READ]).await;

    let (status, body) = send(
        router_for(&pool),
        get_with_key("/api/v1/servers", &creds.api_key),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 0);
}

#[sqlx::test]
async fn missing_scope_is_forbidden(pool: PgPool) {
    // Module sans `hytale_servers:read`.
    let creds = register_module(&pool, "no-scope", &[]).await;

    let (status, _) = send(
        router_for(&pool),
        get_with_key("/api/v1/servers", &creds.api_key),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn write_without_hmac_is_unauthorized(pool: PgPool) {
    let creds = register_module(&pool, "writer", &[scopes::HYTALE_SERVERS_WRITE]).await;
    let body = serde_json::json!({
        "module_id": creds.id,
        "name": "survie",
        "url": "https://s",
    });

    // POST avec API key seulement, sans signature.
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/servers")
        .header("x-api-key", &creds.api_key)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let (status, _) = send(router_for(&pool), request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn write_with_invalid_signature_is_unauthorized(pool: PgPool) {
    let creds = register_module(&pool, "writer", &[scopes::HYTALE_SERVERS_WRITE]).await;
    let body = serde_json::to_vec(&serde_json::json!({
        "module_id": creds.id,
        "name": "survie",
        "url": "https://s",
    }))
    .unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/servers")
        .header("x-api-key", &creds.api_key)
        .header("x-timestamp", now().to_string())
        .header("x-signature", "00".repeat(32))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let (status, _) = send(router_for(&pool), request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn create_server_with_valid_signature_succeeds(pool: PgPool) {
    let creds = register_module(&pool, "writer", &[scopes::HYTALE_SERVERS_WRITE]).await;
    let body = serde_json::json!({
        "module_id": creds.id,
        "name": "survie",
        "url": "https://s",
    });

    let request = signed_request(
        "POST",
        "/api/v1/servers",
        &creds.api_key,
        &creds.hmac_secret,
        Some(body),
    );

    let (status, response) = send(router_for(&pool), request).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(response["name"], "survie");
    assert_eq!(response["module_id"], creds.id.to_string());
}

#[sqlx::test]
async fn all_modules_can_list_all_servers(pool: PgPool) {
    let owner = register_module(&pool, "owner", &[scopes::HYTALE_SERVERS_WRITE]).await;
    let reader = register_module(&pool, "reader", &[scopes::HYTALE_SERVERS_READ]).await;

    let create = signed_request(
        "POST",
        "/api/v1/servers",
        &owner.api_key,
        &owner.hmac_secret,
        Some(serde_json::json!({
            "module_id": owner.id,
            "name": "survie",
            "url": "https://s",
        })),
    );
    let (created_status, created) = send(router_for(&pool), create).await;
    assert_eq!(created_status, StatusCode::CREATED);
    let server_id = created["id"].as_str().unwrap();

    // Un autre module voit le serveur.
    let (status, body) = send(
        router_for(&pool),
        get_with_key("/api/v1/servers", &reader.api_key),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);

    let (status, detail) = send(
        router_for(&pool),
        get_with_key(&format!("/api/v1/servers/{server_id}"), &reader.api_key),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["id"], server_id);
}

#[sqlx::test]
async fn module_cannot_write_foreign_server(pool: PgPool) {
    let owner = register_module(&pool, "owner", &[scopes::HYTALE_SERVERS_WRITE]).await;
    let intruder = register_module(
        &pool,
        "intruder",
        &[scopes::HYTALE_SERVERS_WRITE, scopes::WHITELIST_WRITE],
    )
    .await;

    let create = signed_request(
        "POST",
        "/api/v1/servers",
        &owner.api_key,
        &owner.hmac_secret,
        Some(serde_json::json!({
            "module_id": owner.id,
            "name": "survie",
            "url": "https://s",
        })),
    );
    let (_, created) = send(router_for(&pool), create).await;
    let server_id = created["id"].as_str().unwrap();

    // L'intrus tente de whitelister sur un serveur qui ne lui appartient pas.
    let attempt = signed_request(
        "POST",
        "/api/v1/whitelists",
        &intruder.api_key,
        &intruder.hmac_secret,
        Some(serde_json::json!({
            "hytale_server_id": server_id,
            "hytale_id": Uuid::now_v7(),
        })),
    );
    let (status, _) = send(router_for(&pool), attempt).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn player_centric_route_filters_by_hytale_id(pool: PgPool) {
    let creds = register_module(
        &pool,
        "webhook",
        &[
            scopes::PLAYER_SESSIONS_WRITE,
            scopes::PLAYER_SESSIONS_READ,
            scopes::HYTALE_SERVERS_WRITE,
        ],
    )
    .await;

    let create = signed_request(
        "POST",
        "/api/v1/servers",
        &creds.api_key,
        &creds.hmac_secret,
        Some(serde_json::json!({
            "module_id": creds.id,
            "name": "survie",
            "url": "https://s",
        })),
    );
    let (_, server) = send(router_for(&pool), create).await;
    let server_id = server["id"].as_str().unwrap();

    let player_a = Uuid::now_v7();
    let player_b = Uuid::now_v7();
    for player in [player_a, player_b] {
        let event = signed_request(
            "POST",
            "/api/v1/webhook",
            &creds.api_key,
            &creds.hmac_secret,
            Some(serde_json::json!({
                "event_type": "session_started",
                "hytale_server_id": server_id,
                "hytale_id": player,
            })),
        );
        let (status, _) = send(router_for(&pool), event).await;
        assert_eq!(status, StatusCode::ACCEPTED);
    }

    // Vue player-centric : uniquement les sessions du joueur A.
    let (status, body) = send(
        router_for(&pool),
        get_with_key(
            &format!("/api/v1/players/{player_a}/sessions"),
            &creds.api_key,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["hytale_id"], player_a.to_string());

    // Filtre par serveur sur la vue globale.
    let (status, body) = send(
        router_for(&pool),
        get_with_key(
            &format!("/api/v1/sessions?hytale_server_id={server_id}"),
            &creds.api_key,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 2);
}

#[sqlx::test]
async fn webhook_rejects_unknown_event_type(pool: PgPool) {
    let creds = register_module(&pool, "webhook", &[scopes::PLAYER_SESSIONS_WRITE]).await;
    let request = signed_request(
        "POST",
        "/api/v1/webhook",
        &creds.api_key,
        &creds.hmac_secret,
        Some(serde_json::json!({
            "event_type": "inconnu",
            "hytale_server_id": Uuid::now_v7(),
            "hytale_id": Uuid::now_v7(),
        })),
    );

    // Le body est invalide : Axum renvoie une erreur client avant le handler.
    let response = router_for(&pool).oneshot(request).await.unwrap();
    assert!(response.status().is_client_error());
}

#[sqlx::test]
async fn admin_can_create_module_and_others_cannot(pool: PgPool) {
    let admin = register_module(&pool, "admin", &[scopes::MODULES_WRITE, scopes::ADMIN]).await;
    let regular = register_module(&pool, "regular", &[]).await;

    let body = serde_json::json!({
        "name": "siteweb",
        "scopes": [scopes::HYTALE_SERVERS_READ],
    });

    let allowed = signed_request(
        "POST",
        "/api/v1/modules",
        &admin.api_key,
        &admin.hmac_secret,
        Some(body.clone()),
    );
    let (status, _) = send(router_for(&pool), allowed).await;
    assert_eq!(status, StatusCode::CREATED);

    let denied = signed_request(
        "POST",
        "/api/v1/modules",
        &regular.api_key,
        &regular.hmac_secret,
        Some(serde_json::json!({ "name": "autre", "scopes": [] })),
    );
    let (status, _) = send(router_for(&pool), denied).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn expired_timestamp_is_unauthorized(pool: PgPool) {
    let creds = register_module(&pool, "writer", &[scopes::HYTALE_SERVERS_WRITE]).await;
    let body = serde_json::to_vec(&serde_json::json!({
        "module_id": creds.id,
        "name": "survie",
        "url": "https://s",
    }))
    .unwrap();

    let stale = (now() - 10_000).to_string();
    let signature = sign(&creds.hmac_secret, &stale, &body);
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/servers")
        .header("x-api-key", &creds.api_key)
        .header("x-timestamp", stale)
        .header("x-signature", signature)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let (status, _) = send(router_for(&pool), request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
