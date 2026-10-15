mod config;
mod crypto;
mod domains;
mod error;
mod http;
mod ingest;
mod shared;
mod state;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod testutil;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use http_body_util::Full;
use hyper::Request;
use hyper::body::Bytes;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::crypto::Crypto;
use crate::shared::logs;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    // Container health probe: the runtime image ships no shell or curl, so the
    // binary probes its own `/health` endpoint. Exit 0 means healthy.
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        let bind_addr = std::env::var("BIND_ADDR")
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "0.0.0.0:3000".to_string());
        let target = bind_addr.replace("0.0.0.0", "127.0.0.1");
        std::process::exit(if run_healthcheck(&target).await { 0 } else { 1 });
    }

    init_tracing();

    let config = Config::from_env().context("invalid configuration")?;
    let crypto = Crypto::from_base64(&config.encryption_key)?;

    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("could not connect to PostgreSQL")?;

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .context("could not run migrations")?;

    seed_admin(&db, &crypto, &config).await?;

    let bind_addr: SocketAddr = config.bind_addr.parse().context("BIND_ADDR is invalid")?;

    let state = AppState {
        db,
        crypto: Arc::new(crypto),
        config: Arc::new(config),
    };

    let app = http::router::build(state.clone());

    domains::queue::worker::spawn(state);

    let listener = TcpListener::bind(bind_addr)
        .await
        .context("could not bind the listen address")?;

    tracing::info!(%bind_addr, "{}", logs::API_CORE_STARTED);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("HTTP server error")?;

    Ok(())
}

/// Probes `GET /health` on `target` (host:port). Healthy only on HTTP 200
/// within a short deadline; any connection error or other status is unhealthy.
async fn run_healthcheck(target: &str) -> bool {
    const TIMEOUT: Duration = Duration::from_secs(5);

    let uri = format!("http://{target}/health");
    let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new()).build_http();

    let probe = async {
        let request = Request::builder()
            .method("GET")
            .uri(&uri)
            .header("Accept", "application/json")
            .body(Full::new(Bytes::new()))?;
        let response = client.request(request).await?;
        anyhow::Ok(response.status().as_u16())
    };

    matches!(tokio::time::timeout(TIMEOUT, probe).await, Ok(Ok(200)))
}

/// Creates the `admin` module at startup when the seed configuration allows it.
async fn seed_admin(db: &sqlx::PgPool, crypto: &Crypto, config: &Config) -> Result<()> {
    let Some(seed) = &config.seed_admin else {
        tracing::warn!("{}", logs::SEED_DISABLED);
        return Ok(());
    };

    match domains::modules::service::seed_admin(
        db,
        crypto,
        &seed.name,
        &seed.api_key,
        &seed.hmac_secret,
    )
    .await
    {
        Ok(Some(row)) => tracing::info!(
            module_id = %row.id,
            name = %row.name,
            "{}",
            logs::ADMIN_SEEDED
        ),
        Ok(None) => tracing::info!(name = %seed.name, "{}", logs::ADMIN_ALREADY_PRESENT),
        Err(err) => tracing::error!(error = %err, "{}", logs::ADMIN_SEED_FAILED),
    }

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("api_core=debug,tower_http=debug,info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}
