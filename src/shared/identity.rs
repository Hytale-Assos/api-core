use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// Identity of the authenticated module, injected into handlers.
///
/// Lives in `shared` (not `http`) so the `service` layer can reason about scopes
/// and ownership without depending on Axum.
#[derive(Debug, Clone)]
pub struct Principal {
    pub id: Uuid,
    pub name: String,
    pub scopes: Vec<String>,
}

impl Principal {
    /// Tells whether the module holds the requested scope.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    /// True if the module is the backoffice (scope `admin`).
    pub fn is_admin(&self) -> bool {
        self.has_scope(crate::shared::scopes::ADMIN)
    }

    /// Checks a scope and returns `403 Forbidden` otherwise.
    pub fn require(&self, scope: &str) -> AppResult<()> {
        if self.has_scope(scope) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::scopes;

    fn principal(scopes: &[&str]) -> Principal {
        Principal {
            id: Uuid::now_v7(),
            name: "test".into(),
            scopes: scopes.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn checks_scopes() {
        let p = principal(&[scopes::PLAYER_SESSIONS_WRITE]);
        assert!(p.has_scope(scopes::PLAYER_SESSIONS_WRITE));
        assert!(!p.has_scope(scopes::WHITELIST_READ));
        assert!(p.require(scopes::PLAYER_SESSIONS_WRITE).is_ok());
        assert!(matches!(
            p.require(scopes::WHITELIST_READ).unwrap_err(),
            AppError::Forbidden
        ));
    }

    #[test]
    fn admin_scope_implies_bypass() {
        assert!(principal(&[scopes::ADMIN]).is_admin());
        assert!(!principal(&[scopes::MODULES_READ]).is_admin());
    }
}
