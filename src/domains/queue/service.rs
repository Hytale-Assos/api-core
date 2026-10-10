use uuid::Uuid;

use crate::error::AppResult;

use super::repo::{self, WhitelistAction};

/// Enqueues a whitelist command in the same transaction as the business write.
pub async fn enqueue(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    action: WhitelistAction,
    hytale_server_id: Uuid,
    hytale_id: Uuid,
) -> AppResult<()> {
    repo::enqueue_tx(tx, Uuid::now_v7(), action, hytale_server_id, hytale_id).await
}
