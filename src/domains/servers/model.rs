use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Hytale server managed by a module.
#[derive(Debug, Clone, FromRow)]
pub struct HytaleServerRow {
    pub id: Uuid,
    pub module_id: Uuid,
    pub name: String,
    pub url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct HytaleServerView {
    pub id: Uuid,
    pub module_id: Uuid,
    pub name: String,
    pub url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<HytaleServerRow> for HytaleServerView {
    fn from(row: HytaleServerRow) -> Self {
        Self {
            id: row.id,
            module_id: row.module_id,
            name: row.name,
            url: row.url,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateServerRequest {
    pub module_id: Uuid,
    pub name: String,
    pub url: String,
}

/// Agent endpoint a server registers with the core (ADR-0016).
///
/// `url` is the agent's control-plane base URL, `token` its bearer token. The
/// token is encrypted at rest (ADR-0009).
#[derive(Debug, Deserialize)]
pub struct RegisterAgentRequest {
    pub url: String,
    pub token: String,
}

/// Pagination parameters for servers.
#[derive(Debug, Deserialize)]
pub struct ServerQuery {
    #[serde(default)]
    pub module_id: Option<Uuid>,
    #[serde(default = "crate::shared::pagination::default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_params_apply_defaults() {
        let query: ServerQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.limit, crate::shared::pagination::DEFAULT_LIMIT);
        assert_eq!(query.offset, 0);
        assert!(query.module_id.is_none());
    }
}
