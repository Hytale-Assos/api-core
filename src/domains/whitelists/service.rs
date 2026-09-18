use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::shared::identity::Principal;

use super::model::{CreateWhitelistRequest, WhitelistView};
use super::repo;
use crate::domains::servers::service as servers_service;

/// Ajoute une entrée de whitelist après vérification de l'ownership du serveur.
pub async fn create(
    pool: &sqlx::PgPool,
    principal: &Principal,
    request: CreateWhitelistRequest,
) -> AppResult<WhitelistView> {
    servers_service::assert_owned(pool, principal, request.hytale_server_id).await?;
    let row = repo::insert(
        pool,
        Uuid::now_v7(),
        request.hytale_server_id,
        request.hytale_id,
    )
    .await?;
    Ok(WhitelistView::from(row))
}

/// Liste paginée de la whitelist (filtres optionnels).
pub async fn list(
    pool: &sqlx::PgPool,
    hytale_server_id: Option<Uuid>,
    hytale_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<WhitelistView>, i64)> {
    let (rows, total) = repo::list(pool, hytale_server_id, hytale_id, limit, offset).await?;
    Ok((rows.into_iter().map(WhitelistView::from).collect(), total))
}

/// Supprime une entrée si elle dépend d'un serveur du principal (ou admin).
pub async fn delete(pool: &sqlx::PgPool, principal: &Principal, id: Uuid) -> AppResult<()> {
    assert_owned(pool, principal, id).await?;
    repo::soft_delete(pool, id).await
}

/// Vérifie qu'une entrée est dans le périmètre du principal.
pub async fn assert_owned(pool: &sqlx::PgPool, principal: &Principal, id: Uuid) -> AppResult<()> {
    if principal.is_admin() {
        return Ok(());
    }
    if repo::is_owned_by(pool, id, principal.id).await? {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::modules::model::CreateModuleRequest;
    use crate::domains::servers::model::CreateServerRequest;
    use crate::domains::servers::service as servers;
    use crate::shared::scopes;
    use crate::testutil::test_crypto;

    async fn seed_principal(pool: &sqlx::PgPool, name: &str, admin: bool) -> Principal {
        let response = crate::domains::modules::service::create(
            pool,
            &test_crypto(),
            CreateModuleRequest {
                name: name.into(),
                description: None,
                scopes: vec![
                    scopes::WHITELIST_READ.to_string(),
                    scopes::WHITELIST_WRITE.to_string(),
                ]
                .into_iter()
                .chain(admin.then(|| scopes::ADMIN.to_string()))
                .collect(),
            },
        )
        .await
        .unwrap();
        Principal {
            id: response.id,
            name: response.name,
            scopes: response.scopes,
        }
    }

    #[sqlx::test]
    async fn idempotent_and_reinsertable(pool: sqlx::PgPool) {
        let principal = seed_principal(&pool, "m", false).await;
        let server = servers::create(
            &pool,
            &principal,
            CreateServerRequest {
                module_id: principal.id,
                name: "srv".into(),
                url: "https://s".into(),
            },
        )
        .await
        .unwrap();
        let player = Uuid::now_v7();

        let first = create(
            &pool,
            &principal,
            CreateWhitelistRequest {
                hytale_server_id: server.id,
                hytale_id: player,
            },
        )
        .await
        .unwrap();
        let again = create(
            &pool,
            &principal,
            CreateWhitelistRequest {
                hytale_server_id: server.id,
                hytale_id: player,
            },
        )
        .await
        .unwrap();
        assert_eq!(first.id, again.id);

        delete(&pool, &principal, first.id).await.unwrap();
        let reinserted = create(
            &pool,
            &principal,
            CreateWhitelistRequest {
                hytale_server_id: server.id,
                hytale_id: player,
            },
        )
        .await
        .unwrap();
        assert_ne!(reinserted.id, first.id);

        let (_, total) = list(&pool, Some(server.id), Some(player), 50, 0)
            .await
            .unwrap();
        assert_eq!(total, 1);
    }

    #[sqlx::test]
    async fn foreign_module_is_forbidden(pool: sqlx::PgPool) {
        let owner = seed_principal(&pool, "owner", false).await;
        let other = seed_principal(&pool, "other", false).await;
        let server = servers::create(
            &pool,
            &owner,
            CreateServerRequest {
                module_id: owner.id,
                name: "srv".into(),
                url: "https://s".into(),
            },
        )
        .await
        .unwrap();
        let entry = create(
            &pool,
            &owner,
            CreateWhitelistRequest {
                hytale_server_id: server.id,
                hytale_id: Uuid::now_v7(),
            },
        )
        .await
        .unwrap();
        assert!(assert_owned(&pool, &owner, entry.id).await.is_ok());
        assert!(matches!(
            assert_owned(&pool, &other, entry.id).await.unwrap_err(),
            AppError::Forbidden
        ));
    }

    #[sqlx::test]
    async fn deleting_missing_row_is_not_found(pool: sqlx::PgPool) {
        let admin = seed_principal(&pool, "admin", true).await;
        let err = delete(&pool, &admin, Uuid::now_v7()).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound));
    }

    #[sqlx::test]
    async fn admin_can_delete_foreign_entry(pool: sqlx::PgPool) {
        let owner = seed_principal(&pool, "owner", false).await;
        let admin = seed_principal(&pool, "admin", true).await;
        let server = servers::create(
            &pool,
            &owner,
            CreateServerRequest {
                module_id: owner.id,
                name: "srv".into(),
                url: "https://s".into(),
            },
        )
        .await
        .unwrap();
        let entry = create(
            &pool,
            &owner,
            CreateWhitelistRequest {
                hytale_server_id: server.id,
                hytale_id: Uuid::now_v7(),
            },
        )
        .await
        .unwrap();

        assert!(delete(&pool, &admin, entry.id).await.is_ok());
    }
}
