use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::shared::map_unique_violation;

use super::model::HytaleServerRow;

pub async fn insert(
    pool: &sqlx::PgPool,
    id: Uuid,
    module_id: Uuid,
    name: &str,
    url: &str,
) -> AppResult<HytaleServerRow> {
    sqlx::query_as::<_, HytaleServerRow>(
        "INSERT INTO hytale_servers (id, module_id, name, url)
         VALUES ($1, $2, $3, $4)
         RETURNING id, module_id, name, url, created_at, updated_at",
    )
    .bind(id)
    .bind(module_id)
    .bind(name)
    .bind(url)
    .fetch_one(pool)
    .await
    .map_err(map_unique_violation)
}

pub async fn fetch_by_id(pool: &sqlx::PgPool, id: Uuid) -> AppResult<HytaleServerRow> {
    sqlx::query_as::<_, HytaleServerRow>(
        "SELECT id, module_id, name, url, created_at, updated_at
         FROM hytale_servers
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

/// Tells whether a server exists and indeed belongs to the given module.
pub async fn is_owned_by(pool: &sqlx::PgPool, server_id: Uuid, module_id: Uuid) -> AppResult<bool> {
    let owned = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
             SELECT 1 FROM hytale_servers
             WHERE id = $1 AND module_id = $2 AND deleted_at IS NULL
         )",
    )
    .bind(server_id)
    .bind(module_id)
    .fetch_one(pool)
    .await?;

    Ok(owned)
}

pub async fn list(
    pool: &sqlx::PgPool,
    module_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<HytaleServerRow>, i64)> {
    let rows = sqlx::query_as::<_, HytaleServerRow>(
        "SELECT id, module_id, name, url, created_at, updated_at
         FROM hytale_servers
         WHERE deleted_at IS NULL
           AND ($1::uuid IS NULL OR module_id = $1)
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(module_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM hytale_servers
         WHERE deleted_at IS NULL AND ($1::uuid IS NULL OR module_id = $1)",
    )
    .bind(module_id)
    .fetch_one(pool)
    .await?;

    Ok((rows, total))
}

pub async fn soft_delete(pool: &sqlx::PgPool, id: Uuid) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE hytale_servers SET deleted_at = now()
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}
