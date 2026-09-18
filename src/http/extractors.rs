use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::error::AppError;
use crate::shared::identity::Principal;

/// Extrait le [`Principal`] inséré par le middleware `module_auth`.
///
/// Implémente `FromRequestParts` pour permettre l'injection directe comme
/// argument de handler : `principal: Principal`.
impl<S> FromRequestParts<S> for Principal
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Principal>()
            .cloned()
            .ok_or(AppError::Unauthorized)
    }
}
