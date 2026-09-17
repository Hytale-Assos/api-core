use std::env;

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    pub encryption_key: String,
    pub admin_token: Option<String>,
    pub max_body_bytes: usize,
    pub hmac_tolerance_secs: i64,
    pub cors_allowed_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

        let database_url = env::var("DATABASE_URL")
            .context("variable d'environnement DATABASE_URL manquante")?;

        let encryption_key = env::var("ENCRYPTION_KEY")
            .context("variable d'environnement ENCRYPTION_KEY manquante (base64, 32 octets)")?;

        let admin_token = env::var("ADMIN_TOKEN").ok().filter(|s| !s.is_empty());

        let max_body_bytes = env::var("MAX_BODY_BYTES")
            .ok()
            .map(|v| v.parse())
            .transpose()
            .context("MAX_BODY_BYTES invalide")?
            .unwrap_or(1_048_576);

        let hmac_tolerance_secs = env::var("HMAC_TOLERANCE_SECS")
            .ok()
            .map(|v| v.parse())
            .transpose()
            .context("HMAC_TOLERANCE_SECS invalide")?
            .unwrap_or(300);

        let cors_allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "*".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(Self {
            bind_addr,
            database_url,
            encryption_key,
            admin_token,
            max_body_bytes,
            hmac_tolerance_secs,
            cors_allowed_origins,
        })
    }
}
