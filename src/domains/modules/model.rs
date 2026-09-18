use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Module registered with the core (website, Hytale plugin, Discord bot...).
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

/// Public view of a module: never exposes a secret.
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

/// Request body to create a module. The core generates the API key and HMAC secret.
#[derive(Debug, Deserialize)]
pub struct CreateModuleRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Response returned on module creation: contains the secrets in the clear,
/// once, for transmission to the module owner.
#[derive(Debug, Serialize)]
pub struct CreateModuleResponse {
    pub id: Uuid,
    pub name: String,
    pub api_key: String,
    pub hmac_secret: String,
    pub scopes: Vec<String>,
}
