use std::sync::Arc;

use crate::config::Config;
use crate::crypto::Crypto;

/// State shared by all Axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub crypto: Arc<Crypto>,
    pub config: Arc<Config>,
}
