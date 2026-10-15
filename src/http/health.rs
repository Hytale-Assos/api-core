use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::error::AppResult;
use crate::state::AppState;

/// `GET /health` - checks that the service and database respond.
pub async fn health(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "database": "up",
        })),
    ))
}
