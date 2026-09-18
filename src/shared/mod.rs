pub mod hmac;
pub mod identity;
pub mod pagination;
pub mod scopes;

use crate::error::AppError;

/// Transforme une violation d'unicité PostgreSQL en `Conflict` lisible.
pub fn map_unique_violation(err: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db_err) = &err
        && db_err.code().as_deref() == Some("23505")
    {
        return AppError::Conflict("ressource déjà existante".into());
    }
    AppError::Database(err)
}
