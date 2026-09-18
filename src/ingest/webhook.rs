use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppResult;
use crate::shared::identity::Principal;
use crate::shared::scopes;
use crate::state::AppState;

use crate::domains::sessions::service as sessions;

/// Événement d'ingestion envoyé par un module. Le core route vers le domaine
/// métier selon la variante et vérifie le scope correspondant.
#[derive(Debug, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum WebhookEvent {
    /// Le joueur vient d'entrer dans le monde.
    SessionStarted {
        hytale_server_id: Uuid,
        hytale_id: Uuid,
        #[serde(default)]
        joined_at: Option<DateTime<Utc>>,
    },
    /// Le joueur a quitté le monde.
    SessionEnded {
        hytale_server_id: Uuid,
        hytale_id: Uuid,
    },
}

impl WebhookEvent {
    /// Scope requis pour traiter l'événement.
    pub fn required_scope(&self) -> &'static str {
        match self {
            WebhookEvent::SessionStarted { .. } | WebhookEvent::SessionEnded { .. } => {
                scopes::PLAYER_SESSIONS_WRITE
            }
        }
    }
}

/// `POST /api/v1/webhook` — réception d'un événement depuis un module authentifié.
///
/// La requête est déjà validée par le middleware `module_auth` (API key + HMAC).
/// L'événement est ensuite routé vers le service du domaine concerné.
pub async fn receive(
    State(state): State<AppState>,
    principal: Principal,
    Json(event): Json<WebhookEvent>,
) -> AppResult<impl IntoResponse> {
    principal.require(event.required_scope())?;

    match event {
        WebhookEvent::SessionStarted {
            hytale_server_id,
            hytale_id,
            joined_at,
        } => {
            let joined_at = joined_at.unwrap_or_else(Utc::now);
            let row = sessions::start(
                &state.db,
                &principal,
                hytale_server_id,
                hytale_id,
                joined_at,
            )
            .await?;

            tracing::info!(
                session_id = %row.id,
                module = %principal.name,
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
            let closed = sessions::end(&state.db, &principal, hytale_server_id, hytale_id).await?;

            tracing::info!(
                module = %principal.name,
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

#[cfg(test)]
mod tests {
    use super::*;

    const SERVER: &str = "01a0a967-cbaa-7746-b9e8-b16daee6d50b";
    const PLAYER: &str = "11111111-1111-7111-8111-111111111111";

    #[test]
    fn deserializes_session_started() {
        let json = format!(
            r#"{{"event_type":"session_started","hytale_server_id":"{SERVER}","hytale_id":"{PLAYER}"}}"#
        );
        let event: WebhookEvent = serde_json::from_str(&json).unwrap();

        match event {
            WebhookEvent::SessionStarted {
                hytale_server_id,
                hytale_id,
                joined_at,
            } => {
                assert_eq!(hytale_server_id, Uuid::parse_str(SERVER).unwrap());
                assert_eq!(hytale_id, Uuid::parse_str(PLAYER).unwrap());
                assert!(joined_at.is_none(), "joined_at est optionnel");
            }
            _ => panic!("variante attendue : SessionStarted"),
        }
    }

    #[test]
    fn deserializes_session_started_with_explicit_joined_at() {
        let json = format!(
            r#"{{"event_type":"session_started","hytale_server_id":"{SERVER}","hytale_id":"{PLAYER}","joined_at":"2026-09-16T08:00:00Z"}}"#
        );
        let event: WebhookEvent = serde_json::from_str(&json).unwrap();

        match event {
            WebhookEvent::SessionStarted { joined_at, .. } => assert!(joined_at.is_some()),
            _ => panic!("variante attendue : SessionStarted"),
        }
    }

    #[test]
    fn deserializes_session_ended() {
        let json = format!(
            r#"{{"event_type":"session_ended","hytale_server_id":"{SERVER}","hytale_id":"{PLAYER}"}}"#
        );
        let event: WebhookEvent = serde_json::from_str(&json).unwrap();

        assert!(matches!(event, WebhookEvent::SessionEnded { .. }));
        assert_eq!(event.required_scope(), scopes::PLAYER_SESSIONS_WRITE);
    }

    #[test]
    fn rejects_unknown_event_type() {
        let json = format!(
            r#"{{"event_type":"inconnu","hytale_server_id":"{SERVER}","hytale_id":"{PLAYER}"}}"#
        );
        assert!(serde_json::from_str::<WebhookEvent>(&json).is_err());
    }

    #[test]
    fn rejects_missing_required_field() {
        let json = format!(r#"{{"event_type":"session_ended","hytale_server_id":"{SERVER}"}}"#);
        assert!(serde_json::from_str::<WebhookEvent>(&json).is_err());
    }
}
