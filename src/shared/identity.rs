use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// Identité du module authentifié, injectée dans les handlers.
///
/// Vit dans `shared` (et non dans `http`) pour que la couche `service` puisse
/// raisonner sur les scopes et l'ownership sans dépendre d'Axum.
#[derive(Debug, Clone)]
pub struct Principal {
    pub id: Uuid,
    pub name: String,
    pub scopes: Vec<String>,
}

impl Principal {
    /// Indique si le module possède le scope demandé.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    /// Vrai si le module est le backoffice (scope `admin`).
    pub fn is_admin(&self) -> bool {
        self.has_scope(crate::shared::scopes::ADMIN)
    }

    /// Vérifie un scope et renvoie `403 Forbidden` sinon.
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
