use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Scopes reconnus par l'API. Le stockage reste du `TEXT` libre (extensible),
/// ces constantes servent de référence côté code et pour le seed admin.
pub mod scopes {
    pub const PLAYER_SESSIONS_READ: &str = "player_sessions:read";
    pub const PLAYER_SESSIONS_WRITE: &str = "player_sessions:write";
    pub const WHITELIST_READ: &str = "whitelist:read";
    pub const WHITELIST_WRITE: &str = "whitelist:write";
}

/// Module enregistré auprès du core (website, plugin Hytale, bot Discord...).
#[derive(Debug, Clone, FromRow)]
pub struct ModuleRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub hmac_secret: String,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Vue publique d'un module : jamais de secret exposé.
#[derive(Debug, Serialize)]
pub struct ModuleView {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_enabled: bool,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Identité du module authentifié, injectée dans les handlers.
#[derive(Debug, Clone)]
pub struct AuthenticatedModule {
    pub id: Uuid,
    pub name: String,
    pub scopes: Vec<String>,
}

impl AuthenticatedModule {
    /// Indique si le module possède le scope demandé.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }
}

/// Corps de requête pour créer un module. Le core génère l'API key et le secret HMAC.
#[derive(Debug, Deserialize)]
pub struct CreateModuleRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Réponse renvoyée à la création d'un module : contient les secrets en clair,
/// une seule fois, pour transmission à l'admin du module.
#[derive(Debug, Serialize)]
pub struct CreateModuleResponse {
    pub id: Uuid,
    pub name: String,
    pub api_key: String,
    pub hmac_secret: String,
    pub scopes: Vec<String>,
}

/// Serveur Hytale géré par un module.
#[derive(Debug, Clone, FromRow)]
pub struct HytaleServerRow {
    pub id: Uuid,
    pub module_id: Uuid,
    pub name: String,
    pub url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct HytaleServerView {
    pub id: Uuid,
    pub module_id: Uuid,
    pub name: String,
    pub url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<HytaleServerRow> for HytaleServerView {
    fn from(row: HytaleServerRow) -> Self {
        Self {
            id: row.id,
            module_id: row.module_id,
            name: row.name,
            url: row.url,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateServerRequest {
    pub module_id: Uuid,
    pub name: String,
    pub url: String,
}

/// Session de connexion d'un joueur à un serveur.
#[derive(Debug, Clone, FromRow)]
pub struct PlayerSessionRow {
    pub id: Uuid,
    pub hytale_server_id: Uuid,
    pub hytale_id: Uuid,
    pub joined_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PlayerSessionView {
    pub id: Uuid,
    pub hytale_server_id: Uuid,
    pub hytale_id: Uuid,
    pub joined_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<PlayerSessionRow> for PlayerSessionView {
    fn from(row: PlayerSessionRow) -> Self {
        Self {
            id: row.id,
            hytale_server_id: row.hytale_server_id,
            hytale_id: row.hytale_id,
            joined_at: row.joined_at,
            ended_at: row.ended_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Entrée de whitelist d'un joueur pour un serveur.
#[derive(Debug, Clone, FromRow)]
pub struct WhitelistRow {
    pub id: Uuid,
    pub hytale_server_id: Uuid,
    pub hytale_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct WhitelistView {
    pub id: Uuid,
    pub hytale_server_id: Uuid,
    pub hytale_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<WhitelistRow> for WhitelistView {
    fn from(row: WhitelistRow) -> Self {
        Self {
            id: row.id,
            hytale_server_id: row.hytale_server_id,
            hytale_id: row.hytale_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateWhitelistRequest {
    pub hytale_server_id: Uuid,
    pub hytale_id: Uuid,
}

/// Événement d'ingestion envoyé par un module. Le core route vers la table
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

/// Réponse paginée générique.
#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Paramètres de pagination/filtrage des sessions.
#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    #[serde(default)]
    pub hytale_server_id: Option<Uuid>,
    #[serde(default)]
    pub hytale_id: Option<Uuid>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

/// Paramètres de pagination/filtrage de la whitelist.
#[derive(Debug, Deserialize)]
pub struct WhitelistQuery {
    #[serde(default)]
    pub hytale_server_id: Option<Uuid>,
    #[serde(default)]
    pub hytale_id: Option<Uuid>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

/// Paramètres de pagination des serveurs.
#[derive(Debug, Deserialize)]
pub struct ServerQuery {
    #[serde(default)]
    pub module_id: Option<Uuid>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVER: &str = "01a0a967-cbaa-7746-b9e8-b16daee6d50b";
    const PLAYER: &str = "11111111-1111-7111-8111-111111111111";

    #[test]
    fn authenticated_module_checks_scopes() {
        let module = AuthenticatedModule {
            id: Uuid::now_v7(),
            name: "plugin".into(),
            scopes: vec![scopes::PLAYER_SESSIONS_WRITE.to_string()],
        };

        assert!(module.has_scope(scopes::PLAYER_SESSIONS_WRITE));
        assert!(!module.has_scope(scopes::WHITELIST_READ));
    }

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

    #[test]
    fn query_params_apply_defaults() {
        let session: SessionQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(session.limit, 50);
        assert_eq!(session.offset, 0);
        assert!(session.hytale_server_id.is_none());
        assert!(session.hytale_id.is_none());

        let server: ServerQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(server.limit, 50);
        assert!(server.module_id.is_none());

        let whitelist: WhitelistQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(whitelist.limit, 50);
        assert!(whitelist.hytale_id.is_none());
    }

    #[test]
    fn default_limit_is_fifty() {
        assert_eq!(default_limit(), 50);
    }
}
