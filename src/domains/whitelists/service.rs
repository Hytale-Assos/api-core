use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::shared::identity::Principal;

use super::model::{CreateWhitelistRequest, WhitelistView};
use super::repo;
use crate::domains::servers::service as servers_service;

/// Adds a whitelist entry after checking server ownership.
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

/// Paginated list of the whitelist (optional filters).
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

/// Deletes an entry if it belongs to a server of the principal (or admin).
pub async fn delete(pool: &sqlx::PgPool, principal: &Principal, id: Uuid) -> AppResult<()> {
    assert_owned(pool, principal, id).await?;
    repo::soft_delete(pool, id).await
}

/// Asserts an entry is within the principal's scope.
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
    use crate::shared::scopes;
    use crate::testutil::fixtures;
    use crate::testutil::{seed_principal, seed_server};

    const READ_WRITE: [&str; 2] = [scopes::WHITELIST_READ, scopes::WHITELIST_WRITE];

    async fn whitelist_principal(pool: &sqlx::PgPool, name: &str, admin: bool) -> Principal {
        let mut scopes = READ_WRITE.to_vec();
        if admin {
            scopes.push(scopes::ADMIN);
        }
        seed_principal(pool, name, &scopes).await
    }

    fn request(server_id: Uuid, player: Uuid) -> CreateWhitelistRequest {
        CreateWhitelistRequest {
            hytale_server_id: server_id,
            hytale_id: player,
        }
    }

    #[sqlx::test]
    async fn idempotent_and_reinsertable(pool: sqlx::PgPool) {
        let principal = whitelist_principal(&pool, fixtures::MODULE, false).await;
        let server = seed_server(&pool, &principal).await;
        let player = Uuid::now_v7();

        let first = create(&pool, &principal, request(server.id, player))
            .await
            .unwrap();
        let again = create(&pool, &principal, request(server.id, player))
            .await
            .unwrap();
        assert_eq!(first.id, again.id);

        delete(&pool, &principal, first.id).await.unwrap();
        let reinserted = create(&pool, &principal, request(server.id, player))
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
        let owner = whitelist_principal(&pool, fixtures::OWNER, false).await;
        let other = whitelist_principal(&pool, fixtures::OTHER, false).await;
        let server = seed_server(&pool, &owner).await;
        let entry = create(&pool, &owner, request(server.id, Uuid::now_v7()))
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
        let admin = whitelist_principal(&pool, fixtures::ADMIN, true).await;
        let err = delete(&pool, &admin, Uuid::now_v7()).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound));
    }

    #[sqlx::test]
    async fn admin_can_delete_foreign_entry(pool: sqlx::PgPool) {
        let owner = whitelist_principal(&pool, fixtures::OWNER, false).await;
        let admin = whitelist_principal(&pool, fixtures::ADMIN, true).await;
        let server = seed_server(&pool, &owner).await;
        let entry = create(&pool, &owner, request(server.id, Uuid::now_v7()))
            .await
            .unwrap();

        assert!(delete(&pool, &admin, entry.id).await.is_ok());
    }
}
