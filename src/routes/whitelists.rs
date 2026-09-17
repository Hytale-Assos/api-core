use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use uuid::Uuid;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{
    scopes, AuthenticatedModule, CreateWhitelistRequest, Page, WhitelistQuery, WhitelistView,
};
use crate::state::AppState;

/// `GET /api/v1/whitelists` — liste paginée de la whitelist.
///
/// Nécessite le scope `whitelist:read`.
pub async fn list(
    State(state): State<AppState>,
    Extension(module): Extension<AuthenticatedModule>,
    Query(query): Query<WhitelistQuery>,
) -> AppResult<impl IntoResponse> {
    if !module.has_scope(scopes::WHITELIST_READ) {
        return Err(AppError::Forbidden);
    }

    let limit = query.limit.clamp(1, 200);
    let offset = query.offset.max(0);

    let (rows, total) = db::list_whitelists(
        &state.db,
        query.hytale_server_id,
        query.hytale_id,
        limit,
        offset,
    )
    .await?;

    let items: Vec<WhitelistView> = rows.into_iter().map(WhitelistView::from).collect();

    Ok(Json(Page {
        items,
        total,
        limit,
        offset,
    }))
}

/// `POST /api/v1/whitelists` — ajoute une entrée de whitelist (module).
///
/// Nécessite le scope `whitelist:write` et un serveur appartenant au module.
pub async fn create(
    State(state): State<AppState>,
    Extension(module): Extension<AuthenticatedModule>,
    Json(body): Json<CreateWhitelistRequest>,
) -> AppResult<impl IntoResponse> {
    if !module.has_scope(scopes::WHITELIST_WRITE) {
        return Err(AppError::Forbidden);
    }

    db::assert_server_owned(&state.db, body.hytale_server_id, module.id).await?;

    let row = db::insert_whitelist(
        &state.db,
        Uuid::now_v7(),
        body.hytale_server_id,
        body.hytale_id,
    )
    .await?;

    tracing::info!(
        whitelist_id = %row.id,
        module = %module.name,
        hytale_id = %row.hytale_id,
        "entrée de whitelist ajoutée"
    );

    Ok((StatusCode::CREATED, Json(WhitelistView::from(row))))
}

/// `DELETE /api/v1/whitelists/{id}` — suppression logique d'une entrée (module).
///
/// Nécessite le scope `whitelist:write` et une entrée rattachée à un serveur du module.
pub async fn delete(
    State(state): State<AppState>,
    Extension(module): Extension<AuthenticatedModule>,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    if !module.has_scope(scopes::WHITELIST_WRITE) {
        return Err(AppError::Forbidden);
    }

    db::assert_whitelist_owned(&state.db, id, module.id).await?;
    db::soft_delete_whitelist(&state.db, id).await?;

    tracing::info!(
        whitelist_id = %id,
        module = %module.name,
        "entrée de whitelist supprimée"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/admin/whitelists` — ajoute une entrée (admin).
pub async fn admin_create(
    State(state): State<AppState>,
    Json(body): Json<CreateWhitelistRequest>,
) -> AppResult<impl IntoResponse> {
    let row = db::insert_whitelist(
        &state.db,
        Uuid::now_v7(),
        body.hytale_server_id,
        body.hytale_id,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(WhitelistView::from(row))))
}

/// `DELETE /api/v1/admin/whitelists/{id}` — suppression logique (admin).
pub async fn admin_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    db::soft_delete_whitelist(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
