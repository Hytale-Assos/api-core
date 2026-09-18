use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::shared::identity::Principal;

use super::model::{CreateServerRequest, HytaleServerView};
use super::repo;

/// Enregistre un serveur Hytale.
///
/// Le scope `admin` permet de créer un serveur pour n'importe quel module, ce
/// qui serait refusé à un module normal (il ne peut créer que pour lui-même).
pub async fn create(
    pool: &sqlx::PgPool,
    principal: &Principal,
    request: CreateServerRequest,
) -> AppResult<HytaleServerView> {
    let name = request.name.trim();
    let url = request.url.trim();
    if name.is_empty() || url.is_empty() {
        return Err(AppError::BadRequest("name et url sont requis".into()));
    }

    if !principal.is_admin() && principal.id != request.module_id {
        return Err(AppError::Forbidden);
    }

    let row = repo::insert(pool, Uuid::now_v7(), request.module_id, name, url).await?;

    Ok(HytaleServerView::from(row))
}

/// Liste paginée des serveurs.
///
/// La lecture est globale : tout module disposant de `hytale_servers:read` voit
/// l'ensemble des serveurs. Seules les écritures sont restreintes à l'ownership.
pub async fn list(
    pool: &sqlx::PgPool,
    module_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<HytaleServerView>, i64)> {
    let (rows, total) = repo::list(pool, module_id, limit, offset).await?;
    Ok((
        rows.into_iter().map(HytaleServerView::from).collect(),
        total,
    ))
}

/// Détail d'un serveur (lecture globale).
pub async fn get(pool: &sqlx::PgPool, id: Uuid) -> AppResult<HytaleServerView> {
    let row = repo::fetch_by_id(pool, id).await?;
    Ok(HytaleServerView::from(row))
}

/// Vérifie qu'un serveur appartient au principal, sauf pour l'admin.
pub async fn assert_owned(
    pool: &sqlx::PgPool,
    principal: &Principal,
    server_id: Uuid,
) -> AppResult<()> {
    if principal.is_admin() {
        // L'admin peut opérer sur n'importe quel serveur existant.
        repo::fetch_by_id(pool, server_id).await?;
        return Ok(());
    }

    if repo::is_owned_by(pool, server_id, principal.id).await? {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

/// Suppression logique d'un serveur (ownership ou admin requis).
pub async fn delete(pool: &sqlx::PgPool, principal: &Principal, id: Uuid) -> AppResult<()> {
    let row = repo::fetch_by_id(pool, id).await?;
    if !principal.is_admin() && row.module_id != principal.id {
        return Err(AppError::Forbidden);
    }
    repo::soft_delete(pool, id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::modules::model::CreateModuleRequest;
    use crate::shared::identity::Principal;
    use crate::testutil::test_crypto;

    async fn seed_principal(pool: &sqlx::PgPool, name: &str, admin: bool) -> Principal {
        let response = crate::domains::modules::service::create(
            pool,
            &test_crypto(),
            CreateModuleRequest {
                name: name.into(),
                description: None,
                scopes: if admin {
                    vec![crate::shared::scopes::ADMIN.to_string()]
                } else {
                    vec![crate::shared::scopes::HYTALE_SERVERS_READ.to_string()]
                },
            },
        )
        .await
        .unwrap();

        Principal {
            id: response.id,
            name: response.name,
            scopes: response.scopes,
        }
    }

    #[sqlx::test]
    async fn any_module_with_read_scope_sees_all_servers(pool: sqlx::PgPool) {
        let owner = seed_principal(&pool, "owner", false).await;
        let other = seed_principal(&pool, "other", false).await;

        let server = create(
            &pool,
            &owner,
            CreateServerRequest {
                module_id: owner.id,
                name: "survie".into(),
                url: "https://s".into(),
            },
        )
        .await
        .unwrap();

        // Lecture globale : un autre module voit le serveur et son détail.
        assert!(get(&pool, server.id).await.is_ok());
        let (servers, total) = list(&pool, None, 50, 0).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(servers.len(), 1);

        // Le filtre `module_id` restreint la liste.
        let (filtered, filtered_total) = list(&pool, Some(other.id), 50, 0).await.unwrap();
        assert_eq!(filtered_total, 0);
        assert!(filtered.is_empty());
    }

    #[sqlx::test]
    async fn module_cannot_create_for_another_module(pool: sqlx::PgPool) {
        let owner = seed_principal(&pool, "owner", false).await;
        let other = seed_principal(&pool, "other", false).await;

        let err = create(
            &pool,
            &owner,
            CreateServerRequest {
                module_id: other.id,
                name: "survie".into(),
                url: "https://s".into(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Forbidden));
    }

    #[sqlx::test]
    async fn duplicate_name_is_conflict(pool: sqlx::PgPool) {
        let owner = seed_principal(&pool, "owner", false).await;
        let request = || CreateServerRequest {
            module_id: owner.id,
            name: "survie".into(),
            url: "https://s".into(),
        };

        create(&pool, &owner, request()).await.unwrap();
        let err = create(&pool, &owner, request()).await.unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[sqlx::test]
    async fn soft_deleted_name_can_be_reused(pool: sqlx::PgPool) {
        let owner = seed_principal(&pool, "owner", false).await;
        let first = create(
            &pool,
            &owner,
            CreateServerRequest {
                module_id: owner.id,
                name: "survie".into(),
                url: "https://a".into(),
            },
        )
        .await
        .unwrap();
        delete(&pool, &owner, first.id).await.unwrap();

        assert!(
            create(
                &pool,
                &owner,
                CreateServerRequest {
                    module_id: owner.id,
                    name: "survie".into(),
                    url: "https://b".into(),
                },
            )
            .await
            .is_ok(),
            "l'index unique partiel autorise le même nom après suppression"
        );
    }

    #[sqlx::test]
    async fn deleting_missing_row_is_not_found(pool: sqlx::PgPool) {
        let admin = seed_principal(&pool, "admin", true).await;
        let err = delete(&pool, &admin, Uuid::now_v7()).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound));
    }
}
