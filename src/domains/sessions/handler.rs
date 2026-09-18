use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::error::AppResult;
use crate::shared::identity::Principal;
use crate::shared::pagination::{Page, normalize};
use crate::shared::scopes;
use crate::state::AppState;

use super::model::SessionQuery;
use super::service;

/// `GET /api/v1/sessions` - paginated list of sessions.
///
/// Required scope: `player_sessions:read`.
pub async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<SessionQuery>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::PLAYER_SESSIONS_READ)?;

    let (limit, offset) = normalize(query.limit, query.offset);
    let (items, total) = service::list(
        &state.db,
        query.hytale_server_id,
        query.hytale_id,
        limit,
        offset,
    )
    .await?;

    Ok(Json(Page {
        items,
        total,
        limit,
        offset,
    }))
}

/// `GET /api/v1/players/{hytale_id}/sessions` - a player's sessions.
///
/// Required scope: `player_sessions:read`. Le `hytale_id` du chemin est un filtre
/// filter, never an identity: authorization stays carried by the module.
pub async fn list_for_player(
    State(state): State<AppState>,
    principal: Principal,
    Path(hytale_id): Path<Uuid>,
    Query(query): Query<SessionQuery>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::PLAYER_SESSIONS_READ)?;

    let (limit, offset) = normalize(query.limit, query.offset);
    let (items, total) = service::list(
        &state.db,
        query.hytale_server_id,
        Some(hytale_id),
        limit,
        offset,
    )
    .await?;

    Ok(Json(Page {
        items,
        total,
        limit,
        offset,
    }))
}
