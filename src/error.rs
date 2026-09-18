use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// User-facing messages, defined in one place.
///
/// Never inline a literal message at the call site: reference these constants
/// so the public error contract stays consistent and easy to change.
pub mod messages {
    // Authentication / authorization
    pub const UNAUTHORIZED: &str = "authentication missing or invalid";
    pub const INVALID_SIGNATURE: &str = "invalid HMAC signature";
    pub const EXPIRED_TIMESTAMP: &str = "request timestamp expired";
    pub const FORBIDDEN: &str = "access denied";
    pub const NOT_FOUND: &str = "resource not found";
    pub const INTERNAL: &str = "internal server error";

    // Request validation
    pub const BAD_REQUEST: &str = "invalid request";
    pub const CONFLICT: &str = "conflict";
    pub const BODY_UNREADABLE: &str = "request body could not be read";
    pub const MODULE_NAME_REQUIRED: &str = "name is required";
    pub const SERVER_NAME_AND_URL_REQUIRED: &str = "name and url are required";
    pub const RESOURCE_ALREADY_EXISTS: &str = "resource already exists";

    // Log messages
    pub const LOG_DATABASE_ERROR: &str = "database error";
    pub const LOG_INTERNAL_ERROR: &str = "internal error";
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{}", messages::UNAUTHORIZED)]
    Unauthorized,

    #[error("{}", messages::INVALID_SIGNATURE)]
    InvalidSignature,

    #[error("{}", messages::EXPIRED_TIMESTAMP)]
    ExpiredTimestamp,

    #[error("{}", messages::FORBIDDEN)]
    Forbidden,

    #[error("{}", messages::NOT_FOUND)]
    NotFound,

    #[error("{0}")]
    Conflict(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    /// HTTP status and public message for this error.
    ///
    /// Public messages are composed here from the `messages` constants so the
    /// API contract stays in one place.
    fn public(&self) -> (StatusCode, String) {
        match self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, messages::UNAUTHORIZED.into()),
            AppError::InvalidSignature => {
                (StatusCode::UNAUTHORIZED, messages::INVALID_SIGNATURE.into())
            }
            AppError::ExpiredTimestamp => {
                (StatusCode::UNAUTHORIZED, messages::EXPIRED_TIMESTAMP.into())
            }
            AppError::Forbidden => (StatusCode::FORBIDDEN, messages::FORBIDDEN.into()),
            AppError::NotFound => (StatusCode::NOT_FOUND, messages::NOT_FOUND.into()),
            AppError::Conflict(detail) => (
                StatusCode::CONFLICT,
                format!("{}: {detail}", messages::CONFLICT),
            ),
            AppError::BadRequest(detail) => (
                StatusCode::BAD_REQUEST,
                format!("{}: {detail}", messages::BAD_REQUEST),
            ),
            AppError::Database(err) => {
                tracing::error!(error = %err, "{}", messages::LOG_DATABASE_ERROR);
                (StatusCode::INTERNAL_SERVER_ERROR, messages::INTERNAL.into())
            }
            AppError::Internal(err) => {
                tracing::error!(error = %err, "{}", messages::LOG_INTERNAL_ERROR);
                (StatusCode::INTERNAL_SERVER_ERROR, messages::INTERNAL.into())
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = self.public();
        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    async fn status_and_body(error: AppError) -> (StatusCode, serde_json::Value) {
        let response = error.into_response();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice(&bytes).unwrap();
        (status, json)
    }

    #[tokio::test]
    async fn maps_client_errors_to_expected_statuses() {
        let cases = [
            (AppError::Unauthorized, StatusCode::UNAUTHORIZED),
            (AppError::InvalidSignature, StatusCode::UNAUTHORIZED),
            (AppError::ExpiredTimestamp, StatusCode::UNAUTHORIZED),
            (AppError::Forbidden, StatusCode::FORBIDDEN),
            (AppError::NotFound, StatusCode::NOT_FOUND),
            (AppError::Conflict("duplicate".into()), StatusCode::CONFLICT),
            (
                AppError::BadRequest("invalid".into()),
                StatusCode::BAD_REQUEST,
            ),
        ];

        for (error, expected) in cases {
            let (status, _) = status_and_body(error).await;
            assert_eq!(status, expected);
        }
    }

    #[tokio::test]
    async fn hides_internal_and_database_details() {
        let internal = AppError::Internal(anyhow::anyhow!("internal secret"));
        let (status, body) = status_and_body(internal).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"], messages::INTERNAL);

        let database = AppError::Database(sqlx::Error::RowNotFound);
        let (status, body) = status_and_body(database).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"], messages::INTERNAL);
    }

    #[tokio::test]
    async fn client_errors_keep_their_message() {
        let (_, body) = status_and_body(AppError::Conflict("already exists".into())).await;
        assert_eq!(body["error"], "conflict: already exists");
    }
}
