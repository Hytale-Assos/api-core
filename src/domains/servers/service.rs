use uuid::Uuid;

use crate::error::{AppError, AppResult, messages};
use crate::shared::identity::Principal;

use super::model::{CreateServerRequest, HytaleServerView};
use super::repo;

/// Registers a Hytale server.
///
/// The `admin` scope allows creating a server for any module, which would be
/// denied to a regular module (it may only create for itself).
pub async fn create(
    pool: &sqlx::PgPool,
    principal: &Principal,
    request: CreateServerRequest,
) -> AppResult<HytaleServerView> {
    let name = request.name.trim();
    let url = request.url.trim();
    if name.is_empty() || url.is_empty() {
        return Err(AppError::BadRequest(
            messages::SERVER_NAME_AND_URL_REQUIRED.into(),
        ));
    }

    if !principal.is_admin() && principal.id != request.module_id {
        return Err(AppError::Forbidden);
    }

    let row = repo::insert(pool, Uuid::now_v7(), request.module_id, name, url).await?;

    Ok(HytaleServerView::from(row))
}

/// Paginated list of servers.
///
/// Reads are global: any module holding `hytale_servers:read` sees all servers.
/// Only writes are restricted to ownership.
pub async fn list(
    pool: &sqlx::PgPool,
    module_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<HytaleServerView>, i64)> {
    let (rows, total) = repo::list(pool, module_id, limit, offset).await?;
    Ok((
        rows.into_iter().map(HytaleServerView::from).collect(),
        total,
    ))
}

/// Details of a server (global read).
pub async fn get(pool: &sqlx::PgPool, id: Uuid) -> AppResult<HytaleServerView> {
    let row = repo::fetch_by_id(pool, id).await?;
    Ok(HytaleServerView::from(row))
}

/// Asserts a server belongs to the principal, except for admin.
pub async fn assert_owned(
    pool: &sqlx::PgPool,
    principal: &Principal,
    server_id: Uuid,
) -> AppResult<()> {
    if principal.is_admin() {
        // Admin may operate on any existing server.
        repo::fetch_by_id(pool, server_id).await?;
        return Ok(());
    }

    if repo::is_owned_by(pool, server_id, principal.id).await? {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

/// Soft delete of a server (ownership or admin required).
pub async fn delete(pool: &sqlx::PgPool, principal: &Principal, id: Uuid) -> AppResult<()> {
    let row = repo::fetch_by_id(pool, id).await?;
    if !principal.is_admin() && row.module_id != principal.id {
        return Err(AppError::Forbidden);
    }
    repo::soft_delete(pool, id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::identity::Principal;
    use crate::testutil::fixtures;
    use crate::testutil::{seed_principal, seed_server, server_request};

    async fn principal(pool: &sqlx::PgPool, name: &str, admin: bool) -> Principal {
        let scopes = if admin {
            [crate::shared::scopes::ADMIN]
        } else {
            [crate::shared::scopes::HYTALE_SERVERS_READ]
        };
        seed_principal(pool, name, &scopes).await
    }

    #[sqlx::test]
    async fn any_module_with_read_scope_sees_all_servers(pool: sqlx::PgPool) {
        let owner = principal(&pool, fixtures::OWNER, false).await;
        let other = principal(&pool, fixtures::OTHER, false).await;

        let server = seed_server(&pool, &owner).await;

        // Global read: another module sees the server and its details.
        assert!(get(&pool, server.id).await.is_ok());
        let (servers, total) = list(&pool, None, 50, 0).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(servers.len(), 1);

        // The `module_id` filter narrows the list.
        let (filtered, filtered_total) = list(&pool, Some(other.id), 50, 0).await.unwrap();
        assert_eq!(filtered_total, 0);
        assert!(filtered.is_empty());
    }

    #[sqlx::test]
    async fn module_cannot_create_for_another_module(pool: sqlx::PgPool) {
        let owner = principal(&pool, fixtures::OWNER, false).await;
        let other = principal(&pool, fixtures::OTHER, false).await;

        let err = create(
            &pool,
            &owner,
            server_request(other.id, fixtures::SERVER_NAME, fixtures::SERVER_URL),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Forbidden));
    }

    #[sqlx::test]
    async fn duplicate_name_is_conflict(pool: sqlx::PgPool) {
        let owner = principal(&pool, fixtures::OWNER, false).await;
        let request = || server_request(owner.id, fixtures::SERVER_NAME, fixtures::SERVER_URL);

        create(&pool, &owner, request()).await.unwrap();
        let err = create(&pool, &owner, request()).await.unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[sqlx::test]
    async fn soft_deleted_name_can_be_reused(pool: sqlx::PgPool) {
        let owner = principal(&pool, fixtures::OWNER, false).await;
        let first = create(
            &pool,
            &owner,
            server_request(owner.id, fixtures::SERVER_NAME, fixtures::SERVER_URL),
        )
        .await
        .unwrap();
        delete(&pool, &owner, first.id).await.unwrap();

        assert!(
            create(
                &pool,
                &owner,
                server_request(owner.id, fixtures::SERVER_NAME, fixtures::SERVER_URL_ALT),
            )
            .await
            .is_ok(),
            "the partial unique index allows the same name after deletion"
        );
    }

    #[sqlx::test]
    async fn deleting_missing_row_is_not_found(pool: sqlx::PgPool) {
        let admin = principal(&pool, fixtures::ADMIN, true).await;
        let err = delete(&pool, &admin, Uuid::now_v7()).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound));
    }
}
