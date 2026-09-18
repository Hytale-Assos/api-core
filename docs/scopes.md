# Scopes — répertoire

Dernière mise à jour : 18 septembre 2026

Un **scope** est une permission nommée accordée à un module (voir
`module_scopes` dans `docs/schema.md`). Il remplace l'ancien champ `kind` et
permet d'autoriser finement un module selon le service qu'il représente
(website, plugin Hytale, bot Discord, backoffice...).

## Format

```
<ressource>:<action>
```

- `ressource` : la table/entité métier concernée (ex. `player_sessions`).
- `action` : `read` ou `write`.

Le stockage est du `TEXT` libre en base (extensible sans migration). Les
constantes de référence vivent dans `src/shared/scopes.rs`.

## Scopes actuels

| Scope | Constante | Accès accordé | Endpoints concernés |
|---|---|---|---|
| `player_sessions:read` | `PLAYER_SESSIONS_READ` | Lire les sessions | `GET /api/v1/sessions`, `GET /api/v1/players/{hytale_id}/sessions` |
| `player_sessions:write` | `PLAYER_SESSIONS_WRITE` | Créer/clôturer des sessions | `POST /api/v1/webhook` (events `session_started`, `session_ended`) |
| `whitelist:read` | `WHITELIST_READ` | Lire la whitelist | `GET /api/v1/whitelists`, `GET /api/v1/players/{hytale_id}/whitelists` |
| `whitelist:write` | `WHITELIST_WRITE` | Ajouter/retirer des entrées | `POST /api/v1/whitelists`, `DELETE /api/v1/whitelists/{id}` |
| `hytale_servers:read` | `HYTALE_SERVERS_READ` | Lister/consulter **tous** les serveurs | `GET /api/v1/servers`, `GET /api/v1/servers/{id}` |
| `hytale_servers:write` | `HYTALE_SERVERS_WRITE` | Créer/supprimer un serveur | `POST /api/v1/servers`, `DELETE /api/v1/servers/{id}` |
| `modules:read` | `MODULES_READ` | Lister/consulter les modules | `GET /api/v1/modules`, `GET /api/v1/modules/{id}` |
| `modules:write` | `MODULES_WRITE` | Enregistrer un module | `POST /api/v1/modules` |
| `admin` | `ADMIN` | Contourne les contrôles d'ownership | backoffice |

## Ownership vs visibilité

Il n'y a plus d'endpoints `/admin/*` protégés par un token séparé : le
backoffice est un **module** disposant du scope `admin` et des scopes `*:read` /
`*:write`.

- **Écriture** : un module ne peut écrire que sur ses propres serveurs
  (`assert_owned`). Le scope `admin` (backoffice) contourne cette restriction.
- **Lecture** : `hytale_servers:read` donne accès à **tous** les serveurs. La
  lecture n'est pas risquée, seules les modifications sont restreintes. Le
  filtre `module_id` permet de restreindre volontairement la liste.

## Routes non concernées par les scopes

| Route | Protection |
|---|---|
| `GET /health` | publique |

Toutes les autres routes exigent `X-Api-Key` (lecture) ou `X-Api-Key` +
signature HMAC (écriture).

## Attribution

- Les scopes sont fournis à la création du module :
  `POST /api/v1/modules` avec `"scopes": ["player_sessions:read", ...]`.
- Le module `admin` est seedé au démarrage si `SEED_ADMIN_API_KEY` et
  `SEED_ADMIN_HMAC_SECRET` sont définis.
- Le middleware `module_auth` (`src/http/middleware.rs`) charge les scopes dans
  le `Principal` (`src/shared/identity.rs`) ; chaque handler vérifie via
  `principal.require(...)` et renvoie `403 Forbidden` sinon.

## Ajouter un scope

1. Ajouter la constante dans `src/shared/scopes.rs`.
2. La vérifier dans le handler concerné (`principal.require(...)`).
3. La documenter dans le tableau ci-dessus.
4. Aucune migration SQL nécessaire (stockage `TEXT`).
