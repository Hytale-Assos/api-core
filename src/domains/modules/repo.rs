use std::collections::HashMap;

use uuid::Uuid;

use crate::crypto::Crypto;
use crate::error::{AppError, AppResult};
use crate::shared::map_unique_violation;

use super::model::ModuleRow;

/// Fetches an active module by its API key (through the deterministic index).
pub async fn fetch_by_api_key(
    pool: &sqlx::PgPool,
    crypto: &Crypto,
    api_key: &str,
) -> AppResult<ModuleRow> {
    let hash = crypto.keyed_hash(api_key);
    sqlx::query_as::<_, ModuleRow>(
        "SELECT id, name, description, hmac_secret, is_enabled, created_at, updated_at
         FROM modules
         WHERE api_key_hash = $1 AND deleted_at IS NULL",
    )
    .bind(hash)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)
}

/// Fetches a module by its id (including disabled ones).
pub async fn fetch_by_id(pool: &sqlx::PgPool, id: Uuid) -> AppResult<ModuleRow> {
    sqlx::query_as::<_, ModuleRow>(
        "SELECT id, name, description, hmac_secret, is_enabled, created_at, updated_at
         FROM modules
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

/// Fetches a module by its name (including disabled ones).
pub async fn fetch_by_name(pool: &sqlx::PgPool, name: &str) -> AppResult<Option<ModuleRow>> {
    let row = sqlx::query_as::<_, ModuleRow>(
        "SELECT id, name, description, hmac_secret, is_enabled, created_at, updated_at
         FROM modules
         WHERE name = $1 AND deleted_at IS NULL",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Lists the scopes attached to a module.
pub async fn fetch_scopes(pool: &sqlx::PgPool, module_id: Uuid) -> AppResult<Vec<String>> {
    let scopes = sqlx::query_scalar::<_, String>(
        "SELECT scope FROM module_scopes WHERE module_id = $1 ORDER BY scope",
    )
    .bind(module_id)
    .fetch_all(pool)
    .await?;
    Ok(scopes)
}

/// Lists scopes for a set of modules (avoids the N+1).
async fn fetch_scopes_for(
    pool: &sqlx::PgPool,
    module_ids: &[Uuid],
) -> AppResult<HashMap<Uuid, Vec<String>>> {
    if module_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT module_id, scope FROM module_scopes
         WHERE module_id = ANY($1)
         ORDER BY scope",
    )
    .bind(module_ids)
    .fetch_all(pool)
    .await?;

    let mut map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (module_id, scope) in rows {
        map.entry(module_id).or_default().push(scope);
    }
    Ok(map)
}

/// Inserts a module and its scopes, encrypting its secrets.
#[allow(clippy::too_many_arguments)]
pub async fn insert(
    pool: &sqlx::PgPool,
    crypto: &Crypto,
    id: Uuid,
    name: &str,
    description: Option<&str>,
    api_key: &str,
    hmac_secret: &str,
    scopes: &[String],
) -> AppResult<(ModuleRow, Vec<String>)> {
    let api_key_hash = crypto.keyed_hash(api_key);
    let encrypted_secret = crypto.encrypt_str(hmac_secret)?;

    let mut tx = pool.begin().await?;

    let row = sqlx::query_as::<_, ModuleRow>(
        "INSERT INTO modules (id, name, description, api_key_hash, hmac_secret)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, name, description, hmac_secret, is_enabled, created_at, updated_at",
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(api_key_hash)
    .bind(encrypted_secret)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_unique_violation)?;

    for scope in scopes {
        sqlx::query("INSERT INTO module_scopes (module_id, scope) VALUES ($1, $2)")
            .bind(id)
            .bind(scope)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    let scopes = fetch_scopes(pool, id).await?;
    Ok((row, scopes))
}

/// Lists all modules with their scopes.
pub async fn list(pool: &sqlx::PgPool) -> AppResult<Vec<(ModuleRow, Vec<String>)>> {
    let rows = sqlx::query_as::<_, ModuleRow>(
        "SELECT id, name, description, hmac_secret, is_enabled, created_at, updated_at
         FROM modules
         WHERE deleted_at IS NULL
         ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let mut scopes = fetch_scopes_for(pool, &ids).await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let s = scopes.remove(&row.id).unwrap_or_default();
            (row, s)
        })
        .collect())
}
