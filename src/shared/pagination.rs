use serde::Serialize;

/// Réponse paginée générique.
#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Borne `limit` dans `[1, 200]` et garantit `offset >= 0`.
pub fn normalize(limit: i64, offset: i64) -> (i64, i64) {
    (limit.clamp(1, 200), offset.max(0))
}

pub const DEFAULT_LIMIT: i64 = 50;

pub fn default_limit() -> i64 {
    DEFAULT_LIMIT
}
