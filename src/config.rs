use std::env;

use anyhow::{Context, Result};

/// Configuration du module `admin` créé au démarrage si absent.
///
/// Facultative : sans elle, aucun module n'est seedé et il faut passer par la
/// base ou un seed manuel pour amorcer le registre.
#[derive(Clone, Debug)]
pub struct SeedAdminConfig {
    pub name: String,
    pub api_key: String,
    pub hmac_secret: String,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    pub encryption_key: String,
    pub max_body_bytes: usize,
    pub hmac_tolerance_secs: i64,
    pub cors_allowed_origins: Vec<String>,
    pub seed_admin: Option<SeedAdminConfig>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

        let database_url =
            env::var("DATABASE_URL").context("variable d'environnement DATABASE_URL manquante")?;

        let encryption_key = env::var("ENCRYPTION_KEY")
            .context("variable d'environnement ENCRYPTION_KEY manquante (base64, 32 octets)")?;

        let seed_admin = seed_admin_from_env()?;

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
            max_body_bytes,
            hmac_tolerance_secs,
            cors_allowed_origins,
            seed_admin,
        })
    }
}

/// Lit `SEED_ADMIN_API_KEY` / `SEED_ADMIN_HMAC_SECRET` (et optionnellement
/// `SEED_ADMIN_NAME`, défaut `admin`). Renvoie `None` si les deux secrets ne
/// sont pas fournis.
fn seed_admin_from_env() -> Result<Option<SeedAdminConfig>> {
    let api_key = env::var("SEED_ADMIN_API_KEY")
        .ok()
        .filter(|s| !s.is_empty());
    let hmac_secret = env::var("SEED_ADMIN_HMAC_SECRET")
        .ok()
        .filter(|s| !s.is_empty());

    match (api_key, hmac_secret) {
        (Some(api_key), Some(hmac_secret)) => {
            let name = env::var("SEED_ADMIN_NAME")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "admin".to_string());
            Ok(Some(SeedAdminConfig {
                name,
                api_key,
                hmac_secret,
            }))
        }
        (None, None) => Ok(None),
        _ => anyhow::bail!(
            "SEED_ADMIN_API_KEY et SEED_ADMIN_HMAC_SECRET doivent être définis ensemble"
        ),
    }
}
