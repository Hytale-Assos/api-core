use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method};
use axum::middleware::Next;
use axum::response::Response;
use chrono::Utc;

use crate::auth::hmac;
use crate::db;
use crate::error::AppError;
use crate::models::AuthenticatedModule;
use crate::state::AppState;

const HEADER_API_KEY: &str = "x-api-key";
const HEADER_TIMESTAMP: &str = "x-timestamp";
const HEADER_SIGNATURE: &str = "x-signature";
const HEADER_ADMIN_TOKEN: &str = "x-admin-token";

/// Authentifie un module via API key + signature HMAC du corps de requête.
///
/// En-têtes attendus :
/// - `X-Api-Key` : la clé du module.
/// - `X-Timestamp` : horodatage Unix (secondes) de la requête.
/// - `X-Signature` : HMAC-SHA256 hex de `"{timestamp}.{body}"`.
pub async fn module_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    if request.method() == Method::GET {
        authenticate_api_key(&state, &mut request).await?;
        return Ok(next.run(request).await);
    }

    let parts = request.headers().clone();
    let timestamp = header_string(&parts, HEADER_TIMESTAMP)?;

    let tolerance = state.config.hmac_tolerance_secs;
    let provided_ts: i64 = timestamp
        .parse()
        .map_err(|_| AppError::ExpiredTimestamp)?;
    let now = Utc::now().timestamp();
    if (now - provided_ts).abs() > tolerance {
        return Err(AppError::ExpiredTimestamp);
    }

    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, state.config.max_body_bytes)
        .await
        .map_err(|_| AppError::BadRequest("corps de requête illisible".into()))?;

    let api_key = parts
        .headers
        .get(HEADER_API_KEY)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    let signature = parts
        .headers
        .get(HEADER_SIGNATURE)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::InvalidSignature)?;

    let module = db::fetch_module_by_api_key(&state.db, &state.crypto, api_key).await?;

    if !module.is_enabled {
        return Err(AppError::Forbidden);
    }

    let secret = state.crypto.decrypt_str(&module.hmac_secret)?;
    if !hmac::verify(&secret, &timestamp, &bytes, signature) {
        return Err(AppError::InvalidSignature);
    }

    let scopes = db::fetch_module_scopes(&state.db, module.id).await?;

    let mut request = Request::from_parts(parts, Body::from(bytes));
    request.extensions_mut().insert(AuthenticatedModule {
        id: module.id,
        name: module.name,
        scopes,
    });

    Ok(next.run(request).await)
}

async fn authenticate_api_key(state: &AppState, request: &mut Request) -> Result<(), AppError> {
    let api_key = request
        .headers()
        .get(HEADER_API_KEY)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let module = db::fetch_module_by_api_key(&state.db, &state.crypto, api_key).await?;

    if !module.is_enabled {
        return Err(AppError::Forbidden);
    }

    let scopes = db::fetch_module_scopes(&state.db, module.id).await?;

    request.extensions_mut().insert(AuthenticatedModule {
        id: module.id,
        name: module.name,
        scopes,
    });

    Ok(())
}

/// Authentifie un administrateur via un token statique (`ADMIN_TOKEN`).
pub async fn admin_auth(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let provided = request
        .headers()
        .get(HEADER_ADMIN_TOKEN)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let expected = state.config.admin_token.as_deref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "ADMIN_TOKEN non configuré : endpoints d'administration désactivés"
        ))
    })?;

    if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return Err(AppError::Unauthorized);
    }

    Ok(next.run(request).await)
}

fn header_string(headers: &HeaderMap, name: &str) -> Result<String, AppError> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or(AppError::Unauthorized)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
