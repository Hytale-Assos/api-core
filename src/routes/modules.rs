use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use rand::RngCore;
use uuid::Uuid;

use crate::db;
use crate::error::AppResult;
use crate::models::{
    CreateModuleRequest, CreateModuleResponse, ModuleView,
};
use crate::state::AppState;

/// `POST /api/v1/admin/modules` — enregistre un nouveau module (admin uniquement).
///
/// Génère une API key et un secret HMAC, renvoyés en clair une seule fois.
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateModuleRequest>,
) -> AppResult<impl IntoResponse> {
    if body.name.trim().is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "name est requis".into(),
        ));
    }

    let (api_key, hmac_secret) = generate_credentials();
    let id = Uuid::now_v7();

    let (row, scopes) = db::insert_module(
        &state.db,
        &state.crypto,
        id,
        body.name.trim(),
        body.description.as_deref(),
        &api_key,
        &hmac_secret,
        &body.scopes,
    )
    .await?;

    tracing::info!(module_id = %row.id, name = %row.name, "module enregistré");

    Ok((
        StatusCode::CREATED,
        Json(CreateModuleResponse {
            id: row.id,
            name: row.name,
            api_key,
            hmac_secret,
            scopes,
        }),
    ))
}

/// `GET /api/v1/admin/modules` — liste les modules enregistrés (admin uniquement).
pub async fn list(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    let rows = db::list_modules(&state.db).await?;
    let modules: Vec<ModuleView> = rows
        .into_iter()
        .map(|(row, scopes)| ModuleView {
            id: row.id,
            name: row.name,
            description: row.description,
            is_enabled: row.is_enabled,
            scopes,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect();
    Ok(Json(modules))
}

/// `GET /api/v1/admin/modules/{id}` — détail d'un module (admin uniquement).
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let row = db::fetch_module_by_id(&state.db, id).await?;
    let scopes = db::fetch_module_scopes(&state.db, id).await?;
    Ok(Json(ModuleView {
        id: row.id,
        name: row.name,
        description: row.description,
        is_enabled: row.is_enabled,
        scopes,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

/// Génère une API key et un secret HMAC aléatoires (32 octets chacun, hex).
fn generate_credentials() -> (String, String) {
    let mut api_key_bytes = [0u8; 32];
    let mut secret_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut api_key_bytes);
    rand::rng().fill_bytes(&mut secret_bytes);
    (hex::encode(api_key_bytes), hex::encode(secret_bytes))
}
