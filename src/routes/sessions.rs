use axum::extract::{Extension, Query, State};
use axum::response::IntoResponse;
use axum::Json;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{scopes, AuthenticatedModule, Page, PlayerSessionView, SessionQuery};
use crate::state::AppState;

/// `GET /api/v1/sessions` — liste paginée des sessions.
///
/// Nécessite le scope `player_sessions:read`.
pub async fn list(
    State(state): State<AppState>,
    Extension(module): Extension<AuthenticatedModule>,
    Query(query): Query<SessionQuery>,
) -> AppResult<impl IntoResponse> {
    if !module.has_scope(scopes::PLAYER_SESSIONS_READ) {
        return Err(AppError::Forbidden);
    }

    let limit = query.limit.clamp(1, 200);
    let offset = query.offset.max(0);

    let (rows, total) = db::list_sessions(
        &state.db,
        query.hytale_server_id,
        query.hytale_id,
        limit,
        offset,
    )
    .await?;

    let items: Vec<PlayerSessionView> = rows.into_iter().map(PlayerSessionView::from).collect();

    Ok(Json(Page {
        items,
        total,
        limit,
        offset,
    }))
}
