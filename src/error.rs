use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("authentification manquante ou invalide")]
    Unauthorized,

    #[error("signature HMAC invalide")]
    InvalidSignature,

    #[error("horodatage de requête expiré")]
    ExpiredTimestamp,

    #[error("accès refusé")]
    Forbidden,

    #[error("ressource introuvable")]
    NotFound,

    #[error("conflit: {0}")]
    Conflict(String),

    #[error("requête invalide: {0}")]
    BadRequest(String),

    #[error("erreur base de données: {0}")]
    Database(#[from] sqlx::Error),

    #[error("erreur interne: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::InvalidSignature => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::ExpiredTimestamp => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Conflict(_) => (StatusCode::CONFLICT, self.to_string()),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Database(err) => {
                tracing::error!(error = %err, "erreur base de données");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "erreur interne du serveur".to_string(),
                )
            }
            AppError::Internal(err) => {
                tracing::error!(error = %err, "erreur interne");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "erreur interne du serveur".to_string(),
                )
            }
        };

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
            (AppError::Conflict("doublon".into()), StatusCode::CONFLICT),
            (
                AppError::BadRequest("invalide".into()),
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
        let internal = AppError::Internal(anyhow::anyhow!("secret interne"));
        let (status, body) = status_and_body(internal).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"], "erreur interne du serveur");

        let database = AppError::Database(sqlx::Error::RowNotFound);
        let (status, body) = status_and_body(database).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"], "erreur interne du serveur");
    }

    #[tokio::test]
    async fn client_errors_keep_their_message() {
        let (_, body) = status_and_body(AppError::Conflict("déjà existant".into())).await;
        assert_eq!(body["error"], "conflit: déjà existant");
    }
}
