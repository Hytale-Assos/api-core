use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::AppResult;
use crate::shared::identity::Principal;

use super::model::{PlayerSessionRow, PlayerSessionView};
use super::repo;
use crate::domains::servers::service as servers_service;

/// Starts a session after checking server ownership.
pub async fn start(
    pool: &sqlx::PgPool,
    principal: &Principal,
    hytale_server_id: Uuid,
    hytale_id: Uuid,
    joined_at: DateTime<Utc>,
) -> AppResult<PlayerSessionRow> {
    servers_service::assert_owned(pool, principal, hytale_server_id).await?;
    repo::start(pool, Uuid::now_v7(), hytale_server_id, hytale_id, joined_at).await
}

/// Ends a player's open session (ownership checked).
pub async fn end(
    pool: &sqlx::PgPool,
    principal: &Principal,
    hytale_server_id: Uuid,
    hytale_id: Uuid,
) -> AppResult<bool> {
    servers_service::assert_owned(pool, principal, hytale_server_id).await?;
    repo::end(pool, hytale_server_id, hytale_id).await
}

/// Paginated list of sessions (optional filters).
pub async fn list(
    pool: &sqlx::PgPool,
    hytale_server_id: Option<Uuid>,
    hytale_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<PlayerSessionView>, i64)> {
    let (rows, total) = repo::list(pool, hytale_server_id, hytale_id, limit, offset).await?;
    Ok((
        rows.into_iter().map(PlayerSessionView::from).collect(),
        total,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::scopes;
    use crate::testutil::fixtures;
    use crate::testutil::{seed_principal, seed_server};

    fn session_scopes() -> [&'static str; 2] {
        [scopes::PLAYER_SESSIONS_READ, scopes::PLAYER_SESSIONS_WRITE]
    }

    #[sqlx::test]
    async fn lifecycle_is_idempotent(pool: sqlx::PgPool) {
        let principal = seed_principal(&pool, fixtures::MODULE, &session_scopes()).await;
        let server = seed_server(&pool, &principal).await;
        let player = Uuid::now_v7();
        let now = Utc::now();

        let first = start(&pool, &principal, server.id, player, now)
            .await
            .unwrap();
        let second = start(&pool, &principal, server.id, player, now)
            .await
            .unwrap();
        assert_eq!(first.id, second.id);

        assert!(end(&pool, &principal, server.id, player).await.unwrap());
        assert!(!end(&pool, &principal, server.id, player).await.unwrap());

        let (sessions, total) = list(&pool, Some(server.id), Some(player), 50, 0)
            .await
            .unwrap();
        assert_eq!(total, 1);
        assert!(sessions[0].ended_at.is_some());
    }

    #[sqlx::test]
    async fn session_can_be_restarted_after_end(pool: sqlx::PgPool) {
        let principal = seed_principal(&pool, fixtures::MODULE, &session_scopes()).await;
        let server = seed_server(&pool, &principal).await;
        let player = Uuid::now_v7();
        let now = Utc::now();

        let first = start(&pool, &principal, server.id, player, now)
            .await
            .unwrap();
        end(&pool, &principal, server.id, player).await.unwrap();
        let second = start(&pool, &principal, server.id, player, now)
            .await
            .unwrap();

        assert_ne!(first.id, second.id, "a new session is created");
        let (_, total) = list(&pool, Some(server.id), Some(player), 50, 0)
            .await
            .unwrap();
        assert_eq!(total, 2);
    }

    #[sqlx::test]
    async fn cannot_start_session_on_foreign_server(pool: sqlx::PgPool) {
        let owner = seed_principal(&pool, fixtures::OWNER, &session_scopes()).await;
        let other = seed_principal(&pool, fixtures::OTHER, &session_scopes()).await;
        let server = seed_server(&pool, &owner).await;

        let err = start(&pool, &other, server.id, Uuid::now_v7(), Utc::now())
            .await
            .unwrap_err();
        assert!(matches!(err, crate::error::AppError::Forbidden));
    }
}
