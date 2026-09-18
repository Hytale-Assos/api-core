use uuid::Uuid;

use crate::error::AppResult;

use super::model::PlayerSessionRow;

/// Ouvre une session. Idempotent : réutilise la session ouverte s'il en existe une.
pub async fn start(
    pool: &sqlx::PgPool,
    id: Uuid,
    hytale_server_id: Uuid,
    hytale_id: Uuid,
    joined_at: chrono::DateTime<chrono::Utc>,
) -> AppResult<PlayerSessionRow> {
    if let Some(existing) = fetch_open(pool, hytale_server_id, hytale_id).await? {
        return Ok(existing);
    }

    sqlx::query_as::<_, PlayerSessionRow>(
        "INSERT INTO player_sessions (id, hytale_server_id, hytale_id, joined_at)
         VALUES ($1, $2, $3, $4)
         RETURNING id, hytale_server_id, hytale_id, joined_at, ended_at, created_at, updated_at",
    )
    .bind(id)
    .bind(hytale_server_id)
    .bind(hytale_id)
    .bind(joined_at)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

pub async fn fetch_open(
    pool: &sqlx::PgPool,
    hytale_server_id: Uuid,
    hytale_id: Uuid,
) -> AppResult<Option<PlayerSessionRow>> {
    let row = sqlx::query_as::<_, PlayerSessionRow>(
        "SELECT id, hytale_server_id, hytale_id, joined_at, ended_at, created_at, updated_at
         FROM player_sessions
         WHERE hytale_server_id = $1 AND hytale_id = $2
           AND ended_at IS NULL AND deleted_at IS NULL
         ORDER BY joined_at DESC
         LIMIT 1",
    )
    .bind(hytale_server_id)
    .bind(hytale_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Clôt la session ouverte d'un joueur. Renvoie `false` si aucune session ouverte.
pub async fn end(pool: &sqlx::PgPool, hytale_server_id: Uuid, hytale_id: Uuid) -> AppResult<bool> {
    let result = sqlx::query(
        "UPDATE player_sessions SET ended_at = now()
         WHERE id = (
             SELECT id FROM player_sessions
             WHERE hytale_server_id = $1 AND hytale_id = $2
               AND ended_at IS NULL AND deleted_at IS NULL
             ORDER BY joined_at DESC
             LIMIT 1
         )",
    )
    .bind(hytale_server_id)
    .bind(hytale_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn list(
    pool: &sqlx::PgPool,
    hytale_server_id: Option<Uuid>,
    hytale_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<PlayerSessionRow>, i64)> {
    let rows = sqlx::query_as::<_, PlayerSessionRow>(
        "SELECT id, hytale_server_id, hytale_id, joined_at, ended_at, created_at, updated_at
         FROM player_sessions
         WHERE deleted_at IS NULL
           AND ($1::uuid IS NULL OR hytale_server_id = $1)
           AND ($2::uuid IS NULL OR hytale_id = $2)
         ORDER BY joined_at DESC
         LIMIT $3 OFFSET $4",
    )
    .bind(hytale_server_id)
    .bind(hytale_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM player_sessions
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
