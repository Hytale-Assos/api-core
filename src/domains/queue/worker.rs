//! Background worker delivering queued management commands to servers.
//!
//! Runs outside the HTTP request path (ADR-0010): it polls the queue, calls
//! the server's registered management endpoint, and retries with backoff. A
//! server that never registered is left pending instead of failing.

use std::time::Duration;

use chrono::Utc;
use http_body_util::Full;
use hyper::Request;
use hyper::body::Bytes;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;

use crate::shared::logs;
use crate::state::AppState;

use super::model::QueueRow;
use super::repo::{self, WhitelistAction};

type HttpClient = Client<hyper_util::client::legacy::connect::HttpConnector, Full<Bytes>>;

/// Spawns the delivery loop. Never returns; failures are logged and retried.
pub fn spawn(state: AppState) {
    let interval = state.config.queue_poll_secs;
    tokio::spawn(async move {
        let client: HttpClient = Client::builder(TokioExecutor::new()).build_http();
        let mut ticker = tokio::time::interval(Duration::from_secs(interval));
        loop {
            ticker.tick().await;
            if let Err(error) = deliver_batch(&state, &client).await {
                tracing::warn!(error = %error, "{}", logs::QUEUE_BATCH_FAILED);
            }
        }
    });
}

async fn deliver_batch(state: &AppState, client: &HttpClient) -> anyhow::Result<()> {
    let batch = repo::fetch_due(&state.db, state.config.queue_batch_size).await?;
    for row in batch {
        deliver_one(state, client, &row).await;
    }
    Ok(())
}

async fn deliver_one(state: &AppState, client: &HttpClient, row: &QueueRow) {
    let agent = match repo::fetch_agent(&state.db, row.hytale_server_id).await {
        Ok(Some(agent)) => agent,
        Ok(None) => {
            wait(state, row, "server not found").await;
            return;
        }
        Err(error) => {
            wait(state, row, &error.to_string()).await;
            return;
        }
    };

    let (Some(url), Some(token)) = (agent.agent_url, agent.agent_token) else {
        // The server has not registered an agent yet: wait without consuming an
        // attempt, otherwise a queued command could be dropped before linking.
        wait(state, row, "server has no registered agent").await;
        return;
    };

    let token = match state.crypto.decrypt_str(&token) {
        Ok(token) => token,
        Err(error) => {
            // The stored secret is unusable: retrying will not help.
            fail_permanently(state, row, &error.to_string()).await;
            return;
        }
    };

    match push(client, &url, &token, row).await {
        Ok(()) => {
            if let Err(error) = repo::mark_sent(&state.db, row.id).await {
                tracing::error!(error = %error, id = %row.id, "{}", logs::QUEUE_MARK_FAILED);
            }
        }
        Err(error) => defer(state, row, &error.to_string()).await,
    }
}

async fn push(
    client: &HttpClient,
    base_url: &str,
    token: &str,
    row: &QueueRow,
) -> anyhow::Result<()> {
    let action = match row.action.as_str() {
        "add" => WhitelistAction::Add,
        _ => WhitelistAction::Remove,
    };

    let (method, uri, body) = match action {
        WhitelistAction::Add => (
            "POST",
            format!("{}/api/v1/whitelist", base_url.trim_end_matches('/')),
            Some(format!("{{\"player\":\"{}\"}}", row.hytale_id)),
        ),
        WhitelistAction::Remove => (
            "DELETE",
            format!(
                "{}/api/v1/whitelist/{}",
                base_url.trim_end_matches('/'),
                row.hytale_id
            ),
            None,
        ),
    };

    let mut builder = Request::builder()
        .method(method)
        .uri(&uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json");
    if body.is_some() {
        builder = builder.header("Content-Type", "application/json");
    }
    let request = builder.body(Full::new(Bytes::from(body.unwrap_or_default())))?;

    let response = client.request(request).await?;
    let status = response.status();
    if status.is_success() {
        tracing::info!(
            server_id = %row.hytale_server_id,
            hytale_id = %row.hytale_id,
            action = action.as_str(),
            "{}",
            logs::QUEUE_DELIVERED
        );
        Ok(())
    } else {
        anyhow::bail!("management endpoint returned HTTP {status}")
    }
}

/// Postpones a command without counting an attempt (transient wait, e.g. the
/// server is not registered yet). Keeps the fixed poll interval as backoff.
async fn wait(state: &AppState, row: &QueueRow, error: &str) {
    let next = Utc::now() + chrono::Duration::seconds(state.config.queue_poll_secs as i64);
    if let Err(db_error) =
        repo::reschedule(&state.db, row.id, row.attempts, error, next, false).await
    {
        tracing::error!(error = %db_error, id = %row.id, "{}", logs::QUEUE_MARK_FAILED);
    }
}

async fn defer(state: &AppState, row: &QueueRow, error: &str) {
    let attempts = row.attempts + 1;
    if attempts as u64 >= state.config.queue_max_attempts {
        fail_permanently(state, row, error).await;
        return;
    }

    let backoff = state.config.queue_backoff_secs * (1u64 << attempts.min(10));
    let next = Utc::now() + chrono::Duration::seconds(backoff as i64);

    if let Err(db_error) = repo::reschedule(&state.db, row.id, attempts, error, next, false).await {
        tracing::error!(error = %db_error, id = %row.id, "{}", logs::QUEUE_MARK_FAILED);
    } else {
        tracing::warn!(
            id = %row.id,
            attempts,
            error,
            "{}",
            logs::QUEUE_RESCHEDULED
        );
    }
}

async fn fail_permanently(state: &AppState, row: &QueueRow, error: &str) {
    if let Err(db_error) =
        repo::reschedule(&state.db, row.id, row.attempts + 1, error, Utc::now(), true).await
    {
        tracing::error!(error = %db_error, id = %row.id, "{}", logs::QUEUE_MARK_FAILED);
    } else {
        tracing::error!(id = %row.id, error, "{}", logs::QUEUE_FAILED);
    }
}
