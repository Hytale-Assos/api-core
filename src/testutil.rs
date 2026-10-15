#![allow(dead_code)]

use std::sync::Arc;

use base64::prelude::{BASE64_STANDARD, Engine as _};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::crypto::Crypto;
use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

/// Shared fixtures reused across tests, to avoid repeating literals.
pub mod fixtures {
    pub const MODULE_DESCRIPTION: &str = "test module";
    pub const SITE_MODULE: &str = "siteweb";
    pub const MODULE: &str = "m";

    pub const SERVER_NAME: &str = "survie";
    pub const SERVER_URL: &str = "https://hytale.example/survie";
    pub const SERVER_URL_ALT: &str = "https://hytale.example/autre";

    // Module names
    pub const ADMIN: &str = "admin";
    pub const PLUGIN: &str = "plugin";
    pub const OWNER: &str = "owner";
    pub const OTHER: &str = "other";
    pub const READER: &str = "reader";
    pub const WRITER: &str = "writer";
    pub const WEBHOOK: &str = "webhook";
    pub const REGULAR: &str = "regular";
    pub const INTRUDER: &str = "intruder";
    pub const NO_SCOPE: &str = "no-scope";
}

/// Test master key (fixed value, never used in production).
pub fn test_crypto() -> Crypto {
    Crypto::from_base64(&BASE64_STANDARD.encode([42u8; 32])).expect("valid test key")
}

/// Configuration de test minimale.
pub fn test_config() -> Config {
    Config {
        bind_addr: "127.0.0.1:0".into(),
        database_url: String::new(),
        encryption_key: String::new(),
        max_body_bytes: 1_048_576,
        hmac_tolerance_secs: 300,
        cors_allowed_origins: vec!["*".into()],
        seed_admin: None,
        queue_poll_secs: 5,
        queue_batch_size: 20,
        queue_max_attempts: 10,
        queue_backoff_secs: 2,
    }
}

/// Test application state built from a `#[sqlx::test]` pool.
pub fn test_state(pool: PgPool) -> AppState {
    AppState {
        db: pool,
        crypto: Arc::new(test_crypto()),
        config: Arc::new(test_config()),
    }
}

/// Inserts a test module and returns its id.
pub async fn seed_module(pool: &PgPool, name: &str, scopes: &[&str]) -> Uuid {
    register_module(pool, name, scopes).await.id
}

/// Credentials of a freshly created module.
pub struct Credentials {
    pub id: Uuid,
    pub api_key: String,
    pub hmac_secret: String,
}

/// Creates a test module and returns its credentials in the clear.
pub async fn register_module(pool: &PgPool, name: &str, scopes: &[&str]) -> Credentials {
    use crate::domains::modules::model::CreateModuleRequest;
    use crate::domains::modules::service;

    let scopes: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();
    let response = service::create(
        pool,
        &test_crypto(),
        CreateModuleRequest {
            name: name.to_string(),
            description: Some(fixtures::MODULE_DESCRIPTION.into()),
            scopes,
        },
    )
    .await
    .expect("test module creation");

    Credentials {
        id: response.id,
        api_key: response.api_key,
        hmac_secret: response.hmac_secret,
    }
}

/// Creates a module with the given scopes and returns its `Principal`.
pub async fn seed_principal(
    pool: &PgPool,
    name: &str,
    scopes: &[&str],
) -> crate::shared::identity::Principal {
    let response = register_module(pool, name, scopes).await;
    crate::shared::identity::Principal {
        id: response.id,
        name: name.to_string(),
        scopes: scopes.iter().map(|s| s.to_string()).collect(),
    }
}

/// Builds a server creation request for the given module.
pub fn server_request(
    module_id: Uuid,
    name: &str,
    url: &str,
) -> crate::domains::servers::model::CreateServerRequest {
    crate::domains::servers::model::CreateServerRequest {
        module_id,
        name: name.into(),
        url: url.into(),
    }
}

/// Creates a server owned by `owner` and returns it.
pub async fn seed_server(
    pool: &PgPool,
    owner: &crate::shared::identity::Principal,
) -> crate::domains::servers::model::HytaleServerView {
    crate::domains::servers::service::create(
        pool,
        owner,
        server_request(owner.id, fixtures::SERVER_NAME, fixtures::SERVER_URL),
    )
    .await
    .expect("test server creation")
}

/// Computes the HMAC signature of a request (`"{timestamp}.{body}"`).
pub fn sign(secret: &str, timestamp: &str, body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key of arbitrary size");
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}
