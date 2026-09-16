# Scopes — répertoire

Dernière mise à jour : 16 septembre 2026

Un **scope** est une permission nommée accordée à un module (voir
`module_scopes` dans `docs/schema.md`). Il remplace l'ancien champ `kind` et
permet d'autoriser finement un module selon le service qu'il représente
(website, plugin Hytale, bot Discord...).

## Format

```
<ressource>:<action>
```

- `ressource` : la table/entité métier concernée (ex. `player_sessions`).
- `action` : `read` ou `write`.

Le stockage est du `TEXT` libre en base (extensible sans migration). Les
constantes de référence vivent dans `src/models.rs` (`pub mod scopes`).

## Scopes actuels

| Scope | Constante | Accès accordé | Endpoints concernés |
|---|---|---|---|
| `player_sessions:read` | `PLAYER_SESSIONS_READ` | Lire les sessions | `GET /api/v1/sessions` |
| `player_sessions:write` | `PLAYER_SESSIONS_WRITE` | Créer/clôturer des sessions | `POST /api/v1/webhook` (events `session_started`, `session_ended`) |
| `whitelist:read` | `WHITELIST_READ` | Lire la whitelist | `GET /api/v1/whitelists` |
| `whitelist:write` | `WHITELIST_WRITE` | Ajouter/retirer des entrées | `POST /api/v1/whitelists`, `DELETE /api/v1/whitelists/{id}` |

### Portée des écritures de whitelist

Un module ne peut écrire que sur les serveurs qui **lui appartiennent** :
`POST` vérifie `hytale_server_id` via `assert_server_owned`, et `DELETE`
vérifie l'entrée via `assert_whitelist_owned`. Sinon → `403 Forbidden`.

L'administration (`/api/v1/admin/whitelists`) reste protégée par `ADMIN_TOKEN`
et n'exige pas de scope.

## Routes non concernées par les scopes

| Route | Protection |
|---|---|
| `GET /health` | publique |
| `/api/v1/admin/*` | `X-Admin-Token` (`ADMIN_TOKEN`), pas de scope |

## Attribution

- Les scopes sont fournis à la création du module :
  `POST /api/v1/admin/modules` avec `"scopes": ["player_sessions:read", ...]`.
- Ils sont renvoyés par `GET /api/v1/admin/modules` et
  `GET /api/v1/admin/modules/{id}`.
- Le middleware `module_auth` (`src/auth/middleware.rs`) les charge dans
  `AuthenticatedModule` ; chaque handler vérifie via `has_scope(...)` et
  renvoie `403 Forbidden` sinon.

## Ajouter un scope

1. Ajouter la constante dans `src/models.rs` (`pub mod scopes`).
2. Vérifier le scope dans le handler concerné (`module.has_scope(...)`).
3. Le documenter dans le tableau ci-dessus.
4. Aucune migration SQL nécessaire (stockage `TEXT`).
