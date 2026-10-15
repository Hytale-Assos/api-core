use uuid::Uuid;

use crate::error::AppResult;
use crate::shared::identity::Principal;

use super::model::{CreateWhitelistRequest, WhitelistView};
use super::repo;
use crate::domains::queue::repo as queue_repo;
use crate::domains::queue::service as queue_service;
use crate::domains::servers::service as servers_service;

/// Adds a whitelist entry.
///
/// The whitelist is **self-service** (ADR-0016): any module holding
/// `whitelist:write` may add an entry for any existing server, so a member can
/// whitelist themselves on a server they do not own. The server must exist
/// (referential integrity), which is the only check performed here.
///
/// A `whitelist_queue` command is enqueued in the same transaction so the
/// owning server can be told to grant join access.
pub async fn create(
    pool: &sqlx::PgPool,
    _principal: &Principal,
    request: CreateWhitelistRequest,
) -> AppResult<WhitelistView> {
    // Fails with `NotFound` when the server does not exist (or is soft-deleted).
    servers_service::get(pool, request.hytale_server_id).await?;

    let mut tx = pool.begin().await?;
    let row = repo::insert_tx(
        &mut tx,
        Uuid::now_v7(),
        request.hytale_server_id,
        request.hytale_id,
    )
    .await?;
    queue_service::enqueue(
        &mut tx,
        queue_repo::WhitelistAction::Add,
        request.hytale_server_id,
        request.hytale_id,
    )
    .await?;
    tx.commit().await?;

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

/// Deletes an entry.
///
/// Self-service (ADR-0016): any module holding `whitelist:write` may remove an
/// entry, without ownership. A `whitelist_queue` command is enqueued in the
/// same transaction so the owning server can be told to revoke join access.
pub async fn delete(pool: &sqlx::PgPool, _principal: &Principal, id: Uuid) -> AppResult<()> {
    let row = repo::fetch_by_id(pool, id).await?;

    let mut tx = pool.begin().await?;
    repo::soft_delete_tx(&mut tx, id).await?;
    queue_service::enqueue(
        &mut tx,
        queue_repo::WhitelistAction::Remove,
        row.hytale_server_id,
        row.hytale_id,
    )
    .await?;
    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
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
    async fn another_module_can_whitelist_on_foreign_server(pool: sqlx::PgPool) {
        // Self-service (ADR-0016): a module without ownership may whitelist.
        let owner = whitelist_principal(&pool, fixtures::OWNER, false).await;
        let other = whitelist_principal(&pool, fixtures::OTHER, false).await;
        let server = seed_server(&pool, &owner).await;

        let entry = create(&pool, &other, request(server.id, Uuid::now_v7()))
            .await
            .unwrap();
        assert!(delete(&pool, &other, entry.id).await.is_ok());
    }

    #[sqlx::test]
    async fn creating_on_missing_server_is_not_found(pool: sqlx::PgPool) {
        let principal = whitelist_principal(&pool, fixtures::MODULE, false).await;
        let err = create(&pool, &principal, request(Uuid::now_v7(), Uuid::now_v7()))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound));
    }

    #[sqlx::test]
    async fn deleting_missing_row_is_not_found(pool: sqlx::PgPool) {
        let admin = whitelist_principal(&pool, fixtures::ADMIN, true).await;
        let err = delete(&pool, &admin, Uuid::now_v7()).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound));
    }

    #[sqlx::test]
    async fn create_enqueues_a_queue_add_command(pool: sqlx::PgPool) {
        let principal = whitelist_principal(&pool, fixtures::MODULE, false).await;
        let server = seed_server(&pool, &principal).await;
        let player = Uuid::now_v7();

        create(&pool, &principal, request(server.id, player))
            .await
            .unwrap();

        let (action, count): (String, i64) = sqlx::query_as(
            "SELECT action, COUNT(*) FROM whitelist_queue
             WHERE status = 'pending' AND hytale_id = $1 GROUP BY action",
        )
        .bind(player)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(action, "add");
        assert_eq!(count, 1);
    }

    #[sqlx::test]
    async fn delete_enqueues_a_queue_remove_command(pool: sqlx::PgPool) {
        let principal = whitelist_principal(&pool, fixtures::MODULE, false).await;
        let server = seed_server(&pool, &principal).await;
        let player = Uuid::now_v7();

        let entry = create(&pool, &principal, request(server.id, player))
            .await
            .unwrap();
        delete(&pool, &principal, entry.id).await.unwrap();

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM whitelist_queue
             WHERE status = 'pending' AND hytale_id = $1 AND action = 'remove'",
        )
        .bind(player)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(count, 1);
    }
}
