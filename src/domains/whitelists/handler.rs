use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::error::AppResult;
use crate::shared::identity::Principal;
use crate::shared::pagination::{Page, normalize};
use crate::shared::{logs, scopes};
use crate::state::AppState;

use super::model::{CreateWhitelistRequest, WhitelistQuery};
use super::service;

/// `GET /api/v1/whitelists` - paginated list of the whitelist.
///
/// Required scope: `whitelist:read`.
pub async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<WhitelistQuery>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::WHITELIST_READ)?;

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

/// `GET /api/v1/players/{hytale_id}/whitelists` - a player's whitelist.
///
/// Required scope: `whitelist:read`.
pub async fn list_for_player(
    State(state): State<AppState>,
    principal: Principal,
    Path(hytale_id): Path<Uuid>,
    Query(query): Query<WhitelistQuery>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::WHITELIST_READ)?;

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

/// `POST /api/v1/whitelists` - adds a whitelist entry.
///
/// Required scope: `whitelist:write`. The server must belong to the module.
pub async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateWhitelistRequest>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::WHITELIST_WRITE)?;

    let entry = service::create(&state.db, &principal, body).await?;

    tracing::info!(
        whitelist_id = %entry.id,
        module = %principal.name,
        hytale_id = %entry.hytale_id,
        "{}",
        logs::WHITELIST_ENTRY_ADDED
    );

    Ok((StatusCode::CREATED, Json(entry)))
}

/// `DELETE /api/v1/whitelists/{id}` - soft delete.
///
/// Required scope: `whitelist:write`.
pub async fn delete(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::WHITELIST_WRITE)?;

    service::delete(&state.db, &principal, id).await?;

    tracing::info!(
        whitelist_id = %id,
        module = %principal.name,
        "{}",
        logs::WHITELIST_ENTRY_DELETED
    );

    Ok(StatusCode::NO_CONTENT)
}
