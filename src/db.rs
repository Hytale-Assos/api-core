use std::collections::HashMap;

use uuid::Uuid;

use crate::crypto::Crypto;
use crate::error::{AppError, AppResult};
use crate::models::{HytaleServerRow, ModuleRow, PlayerSessionRow, WhitelistRow};

// ---------------------------------------------------------------------------
// Modules
// ---------------------------------------------------------------------------

/// Récupère un module actif par sa clé API (via l'index déterministe).
pub async fn fetch_module_by_api_key(
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

/// Récupère un module (y compris désactivé) par son identifiant.
pub async fn fetch_module_by_id(pool: &sqlx::PgPool, id: Uuid) -> AppResult<ModuleRow> {
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

/// Liste les scopes attachés à un module.
pub async fn fetch_module_scopes(pool: &sqlx::PgPool, module_id: Uuid) -> AppResult<Vec<String>> {
    let scopes = sqlx::query_scalar::<_, String>(
        "SELECT scope FROM module_scopes WHERE module_id = $1 ORDER BY scope",
    )
    .bind(module_id)
    .fetch_all(pool)
    .await?;
    Ok(scopes)
}

/// Liste les scopes pour un ensemble de modules (évite le N+1).
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

/// Insère un module et ses scopes en chiffrant ses secrets.
#[allow(clippy::too_many_arguments)]
pub async fn insert_module(
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
    .await?;

    for scope in scopes {
        sqlx::query("INSERT INTO module_scopes (module_id, scope) VALUES ($1, $2)")
            .bind(id)
            .bind(scope)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    let scopes = fetch_module_scopes(pool, id).await?;
    Ok((row, scopes))
}

/// Liste tous les modules avec leurs scopes.
pub async fn list_modules(pool: &sqlx::PgPool) -> AppResult<Vec<(ModuleRow, Vec<String>)>> {
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

// ---------------------------------------------------------------------------
// Serveurs Hytale
// ---------------------------------------------------------------------------

pub async fn insert_server(
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

pub async fn fetch_server_by_id(pool: &sqlx::PgPool, id: Uuid) -> AppResult<HytaleServerRow> {
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

pub async fn list_servers(
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

/// Vérifie qu'un serveur existe et appartient bien au module fourni.
pub async fn assert_server_owned(
    pool: &sqlx::PgPool,
    server_id: Uuid,
    module_id: Uuid,
) -> AppResult<()> {
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

    if !owned {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

pub async fn soft_delete_server(pool: &sqlx::PgPool, id: Uuid) -> AppResult<()> {
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

// ---------------------------------------------------------------------------
// Sessions joueur
// ---------------------------------------------------------------------------

/// Ouvre une session. Idempotent : réutilise la session ouverte s'il en existe une.
pub async fn start_session(
    pool: &sqlx::PgPool,
    id: Uuid,
    hytale_server_id: Uuid,
    hytale_id: Uuid,
    joined_at: chrono::DateTime<chrono::Utc>,
) -> AppResult<PlayerSessionRow> {
    if let Some(existing) = fetch_open_session(pool, hytale_server_id, hytale_id).await? {
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

async fn fetch_open_session(
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
pub async fn end_session(
    pool: &sqlx::PgPool,
    hytale_server_id: Uuid,
    hytale_id: Uuid,
) -> AppResult<bool> {
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

pub async fn list_sessions(
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

// ---------------------------------------------------------------------------
// Whitelist
// ---------------------------------------------------------------------------

/// Ajoute une entrée de whitelist. Idempotent : renvoie l'entrée active si elle existe.
pub async fn insert_whitelist(
    pool: &sqlx::PgPool,
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
    .fetch_optional(pool)
    .await
    .map_err(map_unique_violation)?;

    match inserted {
        Some(row) => Ok(row),
        None => fetch_active_whitelist(pool, hytale_server_id, hytale_id)
            .await?
            .ok_or(AppError::NotFound),
    }
}

async fn fetch_active_whitelist(
    pool: &sqlx::PgPool,
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
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn list_whitelists(
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

pub async fn soft_delete_whitelist(pool: &sqlx::PgPool, id: Uuid) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE whitelists SET deleted_at = now() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

/// Vérifie qu'une entrée de whitelist appartient à un serveur du module fourni.
pub async fn assert_whitelist_owned(
    pool: &sqlx::PgPool,
    whitelist_id: Uuid,
    module_id: Uuid,
) -> AppResult<()> {
    let owned = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
             SELECT 1
             FROM whitelists w
             JOIN hytale_servers s ON s.id = w.hytale_server_id
             WHERE w.id = $1 AND s.module_id = $2
               AND w.deleted_at IS NULL AND s.deleted_at IS NULL
         )",
    )
    .bind(whitelist_id)
    .bind(module_id)
    .fetch_one(pool)
    .await?;

    if !owned {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Transforme une violation d'unicité en `Conflict` lisible.
fn map_unique_violation(err: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db_err) = &err
        && db_err.code().as_deref() == Some("23505")
    {
        return AppError::Conflict("ressource déjà existante".into());
    }
    AppError::Database(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::scopes;

    fn test_crypto() -> Crypto {
        use base64::prelude::{Engine as _, BASE64_STANDARD};
        Crypto::from_base64(&BASE64_STANDARD.encode([42u8; 32])).unwrap()
    }

    /// Crée un module de test et renvoie son id.
    async fn seed_module(pool: &sqlx::PgPool, name: &str) -> Uuid {
        let crypto = test_crypto();
        let id = Uuid::now_v7();
        let scopes = vec![
            scopes::PLAYER_SESSIONS_READ.to_string(),
            scopes::PLAYER_SESSIONS_WRITE.to_string(),
        ];
        insert_module(
            pool,
            &crypto,
            id,
            name,
            Some("module de test"),
            &format!("api-key-{name}"),
            &format!("secret-{name}"),
            &scopes,
        )
        .await
        .unwrap();
        id
    }

    #[sqlx::test]
    async fn module_roundtrip_with_encrypted_secret(pool: sqlx::PgPool) {
        let crypto = test_crypto();
        let module_id = Uuid::now_v7();
        let scopes = vec![scopes::PLAYER_SESSIONS_WRITE.to_string()];

        let (row, returned_scopes) = insert_module(
            &pool,
            &crypto,
            module_id,
            "plugin",
            Some("desc"),
            "api-key-abc",
            "secret-en-clair",
            &scopes,
        )
        .await
        .unwrap();

        assert_eq!(row.id, module_id);
        assert!(row.is_enabled);
        assert_eq!(returned_scopes, scopes);

        // Le secret est stocké chiffré, pas en clair.
        let stored: String = sqlx::query_scalar("SELECT hmac_secret FROM modules WHERE id = $1")
            .bind(module_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_ne!(stored, "secret-en-clair");
        assert_eq!(crypto.decrypt_str(&stored).unwrap(), "secret-en-clair");

        let fetched = fetch_module_by_api_key(&pool, &crypto, "api-key-abc")
            .await
            .unwrap();
        assert_eq!(fetched.id, module_id);
        assert_eq!(fetched.hmac_secret, stored);

        let scopes = fetch_module_scopes(&pool, module_id).await.unwrap();
        assert_eq!(scopes, vec![scopes::PLAYER_SESSIONS_WRITE.to_string()]);
    }

    #[sqlx::test]
    async fn fetch_module_by_unknown_api_key_is_unauthorized(pool: sqlx::PgPool) {
        let err = fetch_module_by_api_key(&pool, &test_crypto(), "inexistante")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    #[sqlx::test]
    async fn list_modules_returns_scopes(pool: sqlx::PgPool) {
        seed_module(&pool, "a").await;
        seed_module(&pool, "b").await;

        let modules = list_modules(&pool).await.unwrap();
        assert_eq!(modules.len(), 2);
        assert!(modules.iter().all(|(_, scopes)| !scopes.is_empty()));
    }

    #[sqlx::test]
    async fn server_crud_and_ownership(pool: sqlx::PgPool) {
        let owner = seed_module(&pool, "owner").await;
        let other = seed_module(&pool, "other").await;
        let server_id = Uuid::now_v7();

        let server = insert_server(&pool, server_id, owner, "survie", "https://s")
            .await
            .unwrap();
        assert_eq!(server.name, "survie");

        let fetched = fetch_server_by_id(&pool, server_id).await.unwrap();
        assert_eq!(fetched.module_id, owner);

        // Le propriétaire passe, un autre module est refusé.
        assert!(assert_server_owned(&pool, server_id, owner).await.is_ok());
        assert!(matches!(
            assert_server_owned(&pool, server_id, other).await.unwrap_err(),
            AppError::Forbidden
        ));

        let (list, total) = list_servers(&pool, Some(owner), 50, 0).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(list.len(), 1);

        soft_delete_server(&pool, server_id).await.unwrap();
        assert!(matches!(
            fetch_server_by_id(&pool, server_id).await.unwrap_err(),
            AppError::NotFound
        ));
        // Un serveur supprimé n'est plus "possédé".
        assert!(matches!(
            assert_server_owned(&pool, server_id, owner).await.unwrap_err(),
            AppError::Forbidden
        ));
    }

    #[sqlx::test]
    async fn duplicate_server_name_is_conflict(pool: sqlx::PgPool) {
        let module_id = seed_module(&pool, "m").await;
        insert_server(&pool, Uuid::now_v7(), module_id, "survie", "https://a")
            .await
            .unwrap();

        let err = insert_server(&pool, Uuid::now_v7(), module_id, "survie", "https://b")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[sqlx::test]
    async fn soft_deleted_server_name_can_be_reused(pool: sqlx::PgPool) {
        let module_id = seed_module(&pool, "m").await;
        let first = insert_server(&pool, Uuid::now_v7(), module_id, "survie", "https://a")
            .await
            .unwrap();
        soft_delete_server(&pool, first.id).await.unwrap();

        // L'index unique partiel autorise le même nom après suppression logique.
        assert!(
            insert_server(&pool, Uuid::now_v7(), module_id, "survie", "https://b")
                .await
                .is_ok()
        );
    }

    #[sqlx::test]
    async fn session_lifecycle_is_idempotent(pool: sqlx::PgPool) {
        let module_id = seed_module(&pool, "m").await;
        let server_id = Uuid::now_v7();
        insert_server(&pool, server_id, module_id, "srv", "https://s")
            .await
            .unwrap();
        let player = Uuid::now_v7();
        let now = chrono::Utc::now();

        let first = start_session(&pool, Uuid::now_v7(), server_id, player, now)
            .await
            .unwrap();
        let second = start_session(&pool, Uuid::now_v7(), server_id, player, now)
            .await
            .unwrap();
        assert_eq!(first.id, second.id, "une session ouverte est réutilisée");

        assert!(end_session(&pool, server_id, player).await.unwrap());
        assert!(
            !end_session(&pool, server_id, player).await.unwrap(),
            "aucune session ouverte à clôturer la seconde fois"
        );

        let (sessions, total) = list_sessions(&pool, Some(server_id), Some(player), 50, 0)
            .await
            .unwrap();
        assert_eq!(total, 1);
        assert!(sessions[0].ended_at.is_some());
    }

    #[sqlx::test]
    async fn session_can_be_restarted_after_end(pool: sqlx::PgPool) {
        let module_id = seed_module(&pool, "m").await;
        let server_id = Uuid::now_v7();
        insert_server(&pool, server_id, module_id, "srv", "https://s")
            .await
            .unwrap();
        let player = Uuid::now_v7();
        let now = chrono::Utc::now();

        let first = start_session(&pool, Uuid::now_v7(), server_id, player, now)
            .await
            .unwrap();
        end_session(&pool, server_id, player).await.unwrap();
        let second = start_session(&pool, Uuid::now_v7(), server_id, player, now)
            .await
            .unwrap();

        assert_ne!(first.id, second.id, "une nouvelle session est créée");
        let (_, total) = list_sessions(&pool, Some(server_id), Some(player), 50, 0)
            .await
            .unwrap();
        assert_eq!(total, 2);
    }

    #[sqlx::test]
    async fn whitelist_is_idempotent_and_reinsertable(pool: sqlx::PgPool) {
        let module_id = seed_module(&pool, "m").await;
        let server_id = Uuid::now_v7();
        insert_server(&pool, server_id, module_id, "srv", "https://s")
            .await
            .unwrap();
        let player = Uuid::now_v7();

        let first = insert_whitelist(&pool, Uuid::now_v7(), server_id, player)
            .await
            .unwrap();
        let again = insert_whitelist(&pool, Uuid::now_v7(), server_id, player)
            .await
            .unwrap();
        assert_eq!(first.id, again.id, "entrée active renvoyée, pas de doublon");

        soft_delete_whitelist(&pool, first.id).await.unwrap();
        let reinserted = insert_whitelist(&pool, Uuid::now_v7(), server_id, player)
            .await
            .unwrap();
        assert_ne!(
            reinserted.id, first.id,
            "l'index partiel autorise une nouvelle entrée après suppression"
        );

        let (list, total) = list_whitelists(&pool, Some(server_id), Some(player), 50, 0)
            .await
            .unwrap();
        assert_eq!(total, 1, "seule l'entrée active est listée");
        assert_eq!(list[0].id, reinserted.id);
    }

    #[sqlx::test]
    async fn whitelist_write_ownership(pool: sqlx::PgPool) {
        let owner = seed_module(&pool, "owner").await;
        let other = seed_module(&pool, "other").await;
        let server_id = Uuid::now_v7();
        insert_server(&pool, server_id, owner, "srv", "https://s")
            .await
            .unwrap();
        let entry = insert_whitelist(&pool, Uuid::now_v7(), server_id, Uuid::now_v7())
            .await
            .unwrap();

        assert!(assert_whitelist_owned(&pool, entry.id, owner).await.is_ok());
        assert!(matches!(
            assert_whitelist_owned(&pool, entry.id, other).await.unwrap_err(),
            AppError::Forbidden
        ));

        soft_delete_whitelist(&pool, entry.id).await.unwrap();
        assert!(
            assert_whitelist_owned(&pool, entry.id, owner).await.is_err(),
            "une entrée supprimée n'est plus possédée"
        );
    }

    #[sqlx::test]
    async fn soft_delete_missing_rows_is_not_found(pool: sqlx::PgPool) {
        assert!(matches!(
            soft_delete_server(&pool, Uuid::now_v7()).await.unwrap_err(),
            AppError::NotFound
        ));
        assert!(matches!(
            soft_delete_whitelist(&pool, Uuid::now_v7()).await.unwrap_err(),
            AppError::NotFound
        ));
    }
}
