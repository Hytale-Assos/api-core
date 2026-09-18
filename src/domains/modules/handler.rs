use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::error::AppResult;
use crate::shared::identity::Principal;
use crate::shared::{logs, scopes};
use crate::state::AppState;

use super::model::{CreateModuleRequest, ModuleView};
use super::service;

/// `POST /api/v1/modules` - registers a new module.
///
/// Required scope: `modules:write`. Credentials are returned in the clear
/// only once.
pub async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateModuleRequest>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::MODULES_WRITE)?;

    let response = service::create(&state.db, &state.crypto, body).await?;

    tracing::info!(
        module_id = %response.id,
        name = %response.name,
        "{}",
        logs::MODULE_REGISTERED
    );

    Ok((StatusCode::CREATED, Json(response)))
}

/// `GET /api/v1/modules` - lists modules (without secrets).
///
/// Required scope: `modules:read`.
pub async fn list(
    State(state): State<AppState>,
    principal: Principal,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::MODULES_READ)?;

    let modules: Vec<ModuleView> = service::list(&state.db).await?;
    Ok(Json(modules))
}

/// `GET /api/v1/modules/{id}` - details of a module.
///
/// Required scope: `modules:read`.
pub async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::MODULES_READ)?;

    let module = service::get(&state.db, id).await?;
    Ok(Json(module))
}
