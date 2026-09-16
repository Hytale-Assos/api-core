use std::sync::Arc;

use crate::config::Config;
use crate::crypto::Crypto;

/// État partagé par tous les handlers Axum.
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub crypto: Arc<Crypto>,
    pub config: Arc<Config>,
}
