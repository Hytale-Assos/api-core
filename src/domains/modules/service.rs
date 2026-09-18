use rand::RngCore;
use uuid::Uuid;

use crate::crypto::Crypto;
use crate::error::{AppError, AppResult};

use crate::shared::scopes;

use super::model::{CreateModuleRequest, CreateModuleResponse, ModuleRow, ModuleView};
use super::repo;

/// Enregistre un nouveau module en générant ses credentials.
pub async fn create(
    pool: &sqlx::PgPool,
    crypto: &Crypto,
    request: CreateModuleRequest,
) -> AppResult<CreateModuleResponse> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name est requis".into()));
    }

    let (api_key, hmac_secret) = generate_credentials();
    let id = Uuid::now_v7();

    let (row, scopes) = repo::insert(
        pool,
        crypto,
        id,
        name,
        request.description.as_deref(),
        &api_key,
        &hmac_secret,
        &request.scopes,
    )
    .await?;

    Ok(CreateModuleResponse {
        id: row.id,
        name: row.name,
        api_key,
        hmac_secret,
        scopes,
    })
}

/// Liste les modules avec leurs scopes, sans jamais exposer les secrets.
pub async fn list(pool: &sqlx::PgPool) -> AppResult<Vec<ModuleView>> {
    let rows = repo::list(pool).await?;
    Ok(rows
        .into_iter()
        .map(|(row, scopes)| ModuleView::from_row(row, scopes))
        .collect())
}

/// Détail d'un module.
pub async fn get(pool: &sqlx::PgPool, id: Uuid) -> AppResult<ModuleView> {
    let row = repo::fetch_by_id(pool, id).await?;
    let scopes = repo::fetch_scopes(pool, id).await?;
    Ok(ModuleView::from_row(row, scopes))
}

/// Crée le module `admin` s'il n'existe pas encore, à partir de l'environnement.
///
/// Idempotent : ne fait rien si un module nommé `admin` existe déjà. Renvoie le
/// module créé le cas échéant, pour permettre sa journalisation.
pub async fn seed_admin(
    pool: &sqlx::PgPool,
    crypto: &Crypto,
    name: &str,
    api_key: &str,
    hmac_secret: &str,
) -> AppResult<Option<ModuleRow>> {
    if repo::fetch_by_name(pool, name).await?.is_some() {
        return Ok(None);
    }

    let scopes: Vec<String> = scopes::ALL.iter().map(|s| (*s).to_string()).collect();
    let (row, _) = repo::insert(
        pool,
        crypto,
        Uuid::now_v7(),
        name,
        Some("module backoffice initialisé au démarrage"),
        api_key,
        hmac_secret,
        &scopes,
    )
    .await?;

    Ok(Some(row))
}

/// Génère une API key et un secret HMAC aléatoires (32 octets chacun, hex).
fn generate_credentials() -> (String, String) {
    let mut api_key_bytes = [0u8; 32];
    let mut secret_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut api_key_bytes);
    rand::rng().fill_bytes(&mut secret_bytes);
    (hex::encode(api_key_bytes), hex::encode(secret_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::test_crypto;

    #[sqlx::test]
    async fn create_generates_credentials_and_scopes(pool: sqlx::PgPool) {
        let crypto = test_crypto();
        let response = create(
            &pool,
            &crypto,
            CreateModuleRequest {
                name: "plugin".into(),
                description: Some("desc".into()),
                scopes: vec![scopes::PLAYER_SESSIONS_READ.to_string()],
            },
        )
        .await
        .unwrap();

        assert!(!response.api_key.is_empty());
        assert!(!response.hmac_secret.is_empty());
        assert_eq!(response.scopes, vec![scopes::PLAYER_SESSIONS_READ]);

        let fetched = repo::fetch_by_api_key(&pool, &crypto, &response.api_key)
            .await
            .unwrap();
        assert_eq!(fetched.id, response.id);
    }

    #[sqlx::test]
    async fn secret_is_stored_encrypted(pool: sqlx::PgPool) {
        let crypto = test_crypto();
        let response = create(
            &pool,
            &crypto,
            CreateModuleRequest {
                name: "plugin".into(),
                description: None,
                scopes: vec![],
            },
        )
        .await
        .unwrap();

        let stored: String = sqlx::query_scalar("SELECT hmac_secret FROM modules WHERE id = $1")
            .bind(response.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_ne!(
            stored, response.hmac_secret,
            "le secret ne doit pas être en clair"
        );
        assert_eq!(crypto.decrypt_str(&stored).unwrap(), response.hmac_secret);
    }

    #[sqlx::test]
    async fn unknown_api_key_is_unauthorized(pool: sqlx::PgPool) {
        let err = repo::fetch_by_api_key(&pool, &test_crypto(), "inexistante")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    #[sqlx::test]
    async fn list_returns_scopes(pool: sqlx::PgPool) {
        create(
            &pool,
            &test_crypto(),
            CreateModuleRequest {
                name: "a".into(),
                description: None,
                scopes: vec![scopes::WHITELIST_READ.to_string()],
            },
        )
        .await
        .unwrap();

        let modules = list(&pool).await.unwrap();
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].scopes, vec![scopes::WHITELIST_READ]);
    }

    #[sqlx::test]
    async fn create_rejects_empty_name(pool: sqlx::PgPool) {
        let err = create(
            &pool,
            &test_crypto(),
            CreateModuleRequest {
                name: "  ".into(),
                description: None,
                scopes: vec![],
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[sqlx::test]
    async fn seed_admin_is_idempotent(pool: sqlx::PgPool) {
        let crypto = test_crypto();

        let created = seed_admin(&pool, &crypto, "admin", "key", "secret")
            .await
            .unwrap();
        assert!(created.is_some());

        let again = seed_admin(&pool, &crypto, "admin", "key", "secret")
            .await
            .unwrap();
        assert!(again.is_none(), "le seed ne doit pas recréer le module");

        let module = repo::fetch_by_name(&pool, "admin").await.unwrap().unwrap();
        let granted = repo::fetch_scopes(&pool, module.id).await.unwrap();
        assert!(granted.contains(&scopes::ADMIN.to_string()));
    }
}
