use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// A pending management command queued for a server.
#[derive(Debug, Clone, FromRow)]
pub struct QueueRow {
    pub id: Uuid,
    pub hytale_server_id: Uuid,
    pub hytale_id: Uuid,
    pub action: String,
    pub attempts: i32,
}

/// Agent endpoint registered by a server (including the encrypted token).
#[derive(Debug, Clone, FromRow)]
pub struct ServerAgent {
    pub agent_url: Option<String>,
    pub agent_token: Option<String>,
    pub agent_registered_at: Option<DateTime<Utc>>,
}

/// Public view of a server agent: never exposes the bearer token.
#[derive(Debug, Serialize)]
pub struct AgentView {
    pub registered: bool,
    pub url: Option<String>,
    pub registered_at: Option<DateTime<Utc>>,
}

impl From<ServerAgent> for AgentView {
    fn from(agent: ServerAgent) -> Self {
        let registered = agent.agent_url.is_some() && agent.agent_token.is_some();
        Self {
            registered,
            url: agent.agent_url,
            registered_at: agent.agent_registered_at,
        }
    }
}
