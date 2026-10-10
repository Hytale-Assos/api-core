use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::AppResult;

use super::model::{QueueRow, ServerAgent};

/// Management action pushed to a server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhitelistAction {
    Add,
    Remove,
}

impl WhitelistAction {
    pub fn as_str(self) -> &'static str {
        match self {
            WhitelistAction::Add => "add",
            WhitelistAction::Remove => "remove",
        }
    }
}

/// Enqueues a command inside an existing transaction.
///
/// Idempotent per `(server, player, action)` while a previous command of the
/// same kind is still pending, thanks to the partial unique index: a repeated
/// request does not pile up duplicates.
pub async fn enqueue_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    action: WhitelistAction,
    hytale_server_id: Uuid,
    hytale_id: Uuid,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO whitelist_queue (id, hytale_server_id, hytale_id, action)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (hytale_server_id, hytale_id, action) WHERE status = 'pending'
         DO NOTHING",
    )
    .bind(id)
    .bind(hytale_server_id)
    .bind(hytale_id)
    .bind(action.as_str())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Fetches the batch of due commands to deliver.
pub async fn fetch_due(pool: &sqlx::PgPool, limit: i64) -> AppResult<Vec<QueueRow>> {
    let rows = sqlx::query_as::<_, QueueRow>(
        "SELECT id, hytale_server_id, hytale_id, action, attempts
         FROM whitelist_queue
         WHERE status = 'pending' AND next_attempt_at <= now()
         ORDER BY created_at
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Marks a command as successfully delivered.
pub async fn mark_sent(pool: &sqlx::PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query("UPDATE whitelist_queue SET status = 'sent' WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Returns a failed command to the pending queue with backoff.
pub async fn reschedule(
    pool: &sqlx::PgPool,
    id: Uuid,
    attempts: i32,
    error: &str,
    next_attempt_at: DateTime<Utc>,
    permanent: bool,
) -> AppResult<()> {
    let status = if permanent { "failed" } else { "pending" };
    sqlx::query(
        "UPDATE whitelist_queue
         SET status = $5, attempts = $2, last_error = $3, next_attempt_at = $4
         WHERE id = $1",
    )
    .bind(id)
    .bind(attempts)
    .bind(error)
    .bind(next_attempt_at)
    .bind(status)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetches the agent endpoint of a server.
pub async fn fetch_agent(pool: &sqlx::PgPool, server_id: Uuid) -> AppResult<Option<ServerAgent>> {
    let row = sqlx::query_as::<_, ServerAgent>(
        "SELECT agent_url, agent_token, agent_registered_at
         FROM hytale_servers
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(server_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Registers (or replaces) a server's agent endpoint.
pub async fn set_agent(
    pool: &sqlx::PgPool,
    server_id: Uuid,
    agent_url: &str,
    agent_token: &str,
) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE hytale_servers
         SET agent_url = $2, agent_token = $3, agent_registered_at = now()
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(server_id)
    .bind(agent_url)
    .bind(agent_token)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(crate::error::AppError::NotFound);
    }
    Ok(())
}

/// Clears a server's agent registration.
pub async fn clear_agent(pool: &sqlx::PgPool, server_id: Uuid) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE hytale_servers
         SET agent_url = NULL, agent_token = NULL, agent_registered_at = NULL
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(server_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(crate::error::AppError::NotFound);
    }
    Ok(())
}
