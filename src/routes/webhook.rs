use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use uuid::Uuid;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{AuthenticatedModule, WebhookEvent};
use crate::state::AppState;

/// `POST /api/v1/webhook` — réception d'un événement depuis un module authentifié.
///
/// La requête est déjà validée par le middleware `module_auth` (API key + HMAC).
/// Le core route ensuite l'événement vers la table métier correspondante, après
/// vérification du scope requis.
pub async fn receive(
    State(state): State<AppState>,
    Extension(module): Extension<AuthenticatedModule>,
    Json(event): Json<WebhookEvent>,
) -> AppResult<impl IntoResponse> {
    if !module.has_scope(event.required_scope()) {
        return Err(AppError::Forbidden);
    }

    match event {
        WebhookEvent::SessionStarted {
            hytale_server_id,
            hytale_id,
            joined_at,
        } => {
            db::assert_server_owned(&state.db, hytale_server_id, module.id).await?;
            let joined_at = joined_at.unwrap_or_else(chrono::Utc::now);
            let row = db::start_session(
                &state.db,
                Uuid::now_v7(),
                hytale_server_id,
                hytale_id,
                joined_at,
            )
            .await?;

            tracing::info!(
                session_id = %row.id,
                module = %module.name,
                hytale_id = %hytale_id,
                "session démarrée"
            );

            Ok((
                StatusCode::ACCEPTED,
                Json(serde_json::json!({
                    "id": row.id,
                    "status": "accepted",
                })),
            ))
        }
        WebhookEvent::SessionEnded {
            hytale_server_id,
            hytale_id,
        } => {
            db::assert_server_owned(&state.db, hytale_server_id, module.id).await?;
            let closed = db::end_session(&state.db, hytale_server_id, hytale_id).await?;

            tracing::info!(
                module = %module.name,
                hytale_id = %hytale_id,
                closed,
                "session terminée"
            );

            Ok((
                StatusCode::ACCEPTED,
                Json(serde_json::json!({
                    "hytale_id": hytale_id,
                    "status": if closed { "closed" } else { "no_open_session" },
                })),
            ))
        }
    }
}
