use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use uuid::Uuid;

use crate::db;
use crate::error::AppResult;
use crate::models::{CreateServerRequest, HytaleServerView, Page, ServerQuery};
use crate::state::AppState;

/// `POST /api/v1/admin/servers` — enregistre un serveur Hytale (admin).
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateServerRequest>,
) -> AppResult<impl IntoResponse> {
    if body.name.trim().is_empty() || body.url.trim().is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "name et url sont requis".into(),
        ));
    }

    let row = db::insert_server(
        &state.db,
        Uuid::now_v7(),
        body.module_id,
        body.name.trim(),
        body.url.trim(),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(HytaleServerView::from(row)),
    ))
}

/// `GET /api/v1/admin/servers` — liste paginée des serveurs (admin).
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ServerQuery>,
) -> AppResult<impl IntoResponse> {
    let limit = query.limit.clamp(1, 200);
    let offset = query.offset.max(0);

    let (rows, total) =
        db::list_servers(&state.db, query.module_id, limit, offset).await?;

    let items: Vec<HytaleServerView> = rows.into_iter().map(HytaleServerView::from).collect();

    Ok(Json(Page {
        items,
        total,
        limit,
        offset,
    }))
}

/// `GET /api/v1/admin/servers/{id}` — détail d'un serveur (admin).
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let row = db::fetch_server_by_id(&state.db, id).await?;
    Ok(Json(HytaleServerView::from(row)))
}

/// `DELETE /api/v1/admin/servers/{id}` — suppression logique (admin).
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    db::soft_delete_server(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
