mod auth;
mod config;
mod crypto;
mod db;
mod error;
mod models;
mod router;
mod routes;
mod state;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::crypto::Crypto;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    init_tracing();

    let config = Config::from_env().context("configuration invalide")?;
    let crypto = Crypto::from_base64(&config.encryption_key)?;

    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("connexion à PostgreSQL impossible")?;

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .context("exécution des migrations impossible")?;

    let bind_addr: SocketAddr = config
        .bind_addr
        .parse()
        .context("BIND_ADDR invalide")?;

    let state = AppState {
        db,
        crypto: Arc::new(crypto),
        config: Arc::new(config),
    };

    let app = router::build(state);

    let listener = TcpListener::bind(bind_addr)
        .await
        .context("impossible de lier l'adresse d'écoute")?;

    tracing::info!(%bind_addr, "api-core démarrée");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("erreur du serveur HTTP")?;

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("api_core=debug,tower_http=debug,info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}
