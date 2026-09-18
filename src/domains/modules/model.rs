use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Module enregistré auprès du core (website, plugin Hytale, bot Discord...).
#[derive(Debug, Clone, FromRow)]
pub struct ModuleRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub hmac_secret: String,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Vue publique d'un module : jamais de secret exposé.
#[derive(Debug, Serialize)]
pub struct ModuleView {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_enabled: bool,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ModuleView {
    pub fn from_row(row: ModuleRow, scopes: Vec<String>) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            is_enabled: row.is_enabled,
            scopes,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Corps de requête pour créer un module. Le core génère l'API key et le secret HMAC.
#[derive(Debug, Deserialize)]
pub struct CreateModuleRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Réponse renvoyée à la création d'un module : contient les secrets en clair,
/// une seule fois, pour transmission à l'admin du module.
#[derive(Debug, Serialize)]
pub struct CreateModuleResponse {
    pub id: Uuid,
    pub name: String,
    pub api_key: String,
    pub hmac_secret: String,
    pub scopes: Vec<String>,
}
