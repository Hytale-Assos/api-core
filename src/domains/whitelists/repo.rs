use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::shared::map_unique_violation;

use super::model::WhitelistRow;

/// Adds a whitelist entry inside an existing transaction.
///
/// Idempotent: returns the active entry if it already exists.
pub async fn insert_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    hytale_server_id: Uuid,
    hytale_id: Uuid,
) -> AppResult<WhitelistRow> {
    let inserted = sqlx::query_as::<_, WhitelistRow>(
        "INSERT INTO whitelists (id, hytale_server_id, hytale_id)
         VALUES ($1, $2, $3)
         ON CONFLICT (hytale_server_id, hytale_id) WHERE deleted_at IS NULL
         DO NOTHING
         RETURNING id, hytale_server_id, hytale_id, created_at, updated_at",
    )
    .bind(id)
    .bind(hytale_server_id)
    .bind(hytale_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_unique_violation)?;

    match inserted {
        Some(row) => Ok(row),
        None => fetch_active_tx(tx, hytale_server_id, hytale_id)
            .await?
            .ok_or(AppError::NotFound),
    }
}

/// Fetches an active entry by id.
pub async fn fetch_by_id(pool: &sqlx::PgPool, id: Uuid) -> AppResult<WhitelistRow> {
    let row = sqlx::query_as::<_, WhitelistRow>(
        "SELECT id, hytale_server_id, hytale_id, created_at, updated_at
         FROM whitelists
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(row)
}

async fn fetch_active_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    hytale_server_id: Uuid,
    hytale_id: Uuid,
) -> AppResult<Option<WhitelistRow>> {
    let row = sqlx::query_as::<_, WhitelistRow>(
        "SELECT id, hytale_server_id, hytale_id, created_at, updated_at
         FROM whitelists
         WHERE hytale_server_id = $1 AND hytale_id = $2 AND deleted_at IS NULL",
    )
    .bind(hytale_server_id)
    .bind(hytale_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row)
}

pub async fn list(
    pool: &sqlx::PgPool,
    hytale_server_id: Option<Uuid>,
    hytale_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<WhitelistRow>, i64)> {
    let rows = sqlx::query_as::<_, WhitelistRow>(
        "SELECT id, hytale_server_id, hytale_id, created_at, updated_at
         FROM whitelists
         WHERE deleted_at IS NULL
           AND ($1::uuid IS NULL OR hytale_server_id = $1)
           AND ($2::uuid IS NULL OR hytale_id = $2)
         ORDER BY created_at DESC
         LIMIT $3 OFFSET $4",
    )
    .bind(hytale_server_id)
    .bind(hytale_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM whitelists
         WHERE deleted_at IS NULL
           AND ($1::uuid IS NULL OR hytale_server_id = $1)
           AND ($2::uuid IS NULL OR hytale_id = $2)",
    )
    .bind(hytale_server_id)
    .bind(hytale_id)
    .fetch_one(pool)
    .await?;

    Ok((rows, total))
}

pub async fn soft_delete_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE whitelists SET deleted_at = now() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}
