pub mod hmac;
pub mod identity;
pub mod logs;
pub mod pagination;
pub mod scopes;

use crate::error::{AppError, messages};

/// Maps a PostgreSQL unique violation to a readable `Conflict`.
pub fn map_unique_violation(err: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db_err) = &err
        && db_err.code().as_deref() == Some("23505")
    {
        return AppError::Conflict(messages::RESOURCE_ALREADY_EXISTS.into());
    }
    AppError::Database(err)
}
