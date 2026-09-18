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

/// Clé maître de test (valeur fixe, jamais utilisée en production).
pub fn test_crypto() -> Crypto {
    Crypto::from_base64(&BASE64_STANDARD.encode([42u8; 32])).expect("clé de test valide")
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
    }
}

/// État applicatif de test à partir d'un pool `#[sqlx::test]`.
pub fn test_state(pool: PgPool) -> AppState {
    AppState {
        db: pool,
        crypto: Arc::new(test_crypto()),
        config: Arc::new(test_config()),
    }
}

/// Insère un module de test et renvoie son id.
pub async fn seed_module(pool: &PgPool, name: &str, scopes: &[&str]) -> Uuid {
    register_module(pool, name, scopes).await.id
}

/// Identifiants d'un module fraîchement créé.
pub struct Credentials {
    pub id: Uuid,
    pub api_key: String,
    pub hmac_secret: String,
}

/// Crée un module de test et renvoie ses credentials en clair.
pub async fn register_module(pool: &PgPool, name: &str, scopes: &[&str]) -> Credentials {
    use crate::domains::modules::model::CreateModuleRequest;
    use crate::domains::modules::service;

    let scopes: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();
    let response = service::create(
        pool,
        &test_crypto(),
        CreateModuleRequest {
            name: name.to_string(),
            description: Some("module de test".into()),
            scopes,
        },
    )
    .await
    .expect("création du module de test");

    Credentials {
        id: response.id,
        api_key: response.api_key,
        hmac_secret: response.hmac_secret,
    }
}

/// Calcule la signature HMAC d'une requête (`"{timestamp}.{body}"`).
pub fn sign(secret: &str, timestamp: &str, body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("clé HMAC de taille arbitraire");
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}
