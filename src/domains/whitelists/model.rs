use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Whitelist entry of a player for a server.
#[derive(Debug, Clone, FromRow)]
pub struct WhitelistRow {
    pub id: Uuid,
    pub hytale_server_id: Uuid,
    pub hytale_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct WhitelistView {
    pub id: Uuid,
    pub hytale_server_id: Uuid,
    pub hytale_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<WhitelistRow> for WhitelistView {
    fn from(row: WhitelistRow) -> Self {
        Self {
            id: row.id,
            hytale_server_id: row.hytale_server_id,
            hytale_id: row.hytale_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateWhitelistRequest {
    pub hytale_server_id: Uuid,
    pub hytale_id: Uuid,
}

/// Pagination/filter parameters for the whitelist.
#[derive(Debug, Deserialize)]
pub struct WhitelistQuery {
    #[serde(default)]
    pub hytale_server_id: Option<Uuid>,
    #[serde(default)]
    pub hytale_id: Option<Uuid>,
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
        let query: WhitelistQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.limit, crate::shared::pagination::DEFAULT_LIMIT);
        assert_eq!(query.offset, 0);
        assert!(query.hytale_server_id.is_none());
        assert!(query.hytale_id.is_none());
    }
}
