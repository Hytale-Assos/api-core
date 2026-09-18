use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::error::AppResult;
use crate::shared::identity::Principal;
use crate::shared::pagination::{Page, normalize};
use crate::shared::scopes;
use crate::state::AppState;

use super::model::{CreateServerRequest, HytaleServerView, ServerQuery};
use super::service;

/// `POST /api/v1/servers` - registers a Hytale server.
///
/// Required scope: `hytale_servers:write`.
pub async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateServerRequest>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::HYTALE_SERVERS_WRITE)?;

    let server = service::create(&state.db, &principal, body).await?;
    Ok((StatusCode::CREATED, Json(server)))
}

/// `GET /api/v1/servers` - paginated list of servers.
///
/// Required scope: `hytale_servers:read`. Reads are global (all servers),
/// with an optional `module_id` filter.
pub async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<ServerQuery>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::HYTALE_SERVERS_READ)?;

    let (limit, offset) = normalize(query.limit, query.offset);
    let (items, total) = service::list(&state.db, query.module_id, limit, offset).await?;

    Ok(Json(Page {
        items,
        total,
        limit,
        offset,
    }))
}

/// `GET /api/v1/servers/{id}` - details of a server.
///
/// Required scope: `hytale_servers:read`.
pub async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::HYTALE_SERVERS_READ)?;

    let server: HytaleServerView = service::get(&state.db, id).await?;
    Ok(Json(server))
}

/// `DELETE /api/v1/servers/{id}` - soft delete.
///
/// Required scope: `hytale_servers:write`. Ownership required (except admin).
pub async fn delete(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::HYTALE_SERVERS_WRITE)?;

    service::delete(&state.db, &principal, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
