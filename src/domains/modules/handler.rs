use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::error::AppResult;
use crate::shared::identity::Principal;
use crate::shared::scopes;
use crate::state::AppState;

use super::model::{CreateModuleRequest, ModuleView};
use super::service;

/// `POST /api/v1/modules` — enregistre un nouveau module.
///
/// Scope requis : `modules:write`. Les credentials sont renvoyés en clair une
/// seule fois.
pub async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateModuleRequest>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::MODULES_WRITE)?;

    let response = service::create(&state.db, &state.crypto, body).await?;

    tracing::info!(module_id = %response.id, name = %response.name, "module enregistré");

    Ok((StatusCode::CREATED, Json(response)))
}

/// `GET /api/v1/modules` — liste les modules (sans secrets).
///
/// Scope requis : `modules:read`.
pub async fn list(
    State(state): State<AppState>,
    principal: Principal,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::MODULES_READ)?;

    let modules: Vec<ModuleView> = service::list(&state.db).await?;
    Ok(Json(modules))
}

/// `GET /api/v1/modules/{id}` — détail d'un module.
///
/// Scope requis : `modules:read`.
pub async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    principal.require(scopes::MODULES_READ)?;

    let module = service::get(&state.db, id).await?;
    Ok(Json(module))
}
