# API — référence des endpoints

Dernière mise à jour : 16 septembre 2026

Base URL : `http://<host>:3000`. Corps en JSON (UTF-8).

## Authentification

Trois modes selon le groupe de routes :

| Groupe | En-têtes requis |
|---|---|
| `/health` | aucun |
| `/api/v1/admin/*` | `X-Admin-Token: <ADMIN_TOKEN>` |
| `/api/v1/*` (module, GET) | `X-Api-Key: <api_key>` |
| `/api/v1/*` (module, POST/DELETE) | `X-Api-Key`, `X-Timestamp` (Unix secondes), `X-Signature` |

`X-Signature` = HMAC-SHA256 en hexadécimal de `"{X-Timestamp}.{corps brut}"`,
calculé avec le `hmac_secret` du module. Tolérance d'horodatage :
`HMAC_TOLERANCE_SECS` (défaut 300). Signatures et scopes : voir `docs/scopes.md`.

## Format d'erreur

Toute erreur renvoie un JSON :

```json
{ "error": "message lisible" }
```

| Statut | Signification |
|---|---|
| 400 | corps/paramètre invalide |
| 401 | authentification manquante, clé inconnue, signature invalide ou horodatage expiré |
| 403 | module désactivé, scope manquant, ou ressource hors de son périmètre |
| 404 | ressource introuvable |
| 409 | conflit (nom déjà utilisé) |
| 500 | erreur interne (détails jamais exposés) |

## Types partagés

**`Page<T>`** (réponses de liste) :

```json
{ "items": [], "total": 0, "limit": 50, "offset": 0 }
```

**`HytaleServerView`** :

```json
{
  "id": "01a0a967-cbaa-7746-b9e8-b16daee6d50b",
  "module_id": "01a0a967-cb39-76c1-a30a-4983d3633604",
  "name": "survie",
  "url": "https://hytale.example/survie",
  "created_at": "2026-09-16T08:49:06Z",
  "updated_at": "2026-09-16T08:49:06Z"
}
```

**`PlayerSessionView`** :

```json
{
  "id": "01a0a967-f44a-75b5-886d-d03c232ea2cb",
  "hytale_server_id": "01a0a967-cbaa-7746-b9e8-b16daee6d50b",
  "hytale_id": "11111111-1111-7111-8111-111111111111",
  "joined_at": "2026-09-16T08:49:17Z",
  "ended_at": null,
  "created_at": "2026-09-16T08:49:17Z",
  "updated_at": "2026-09-16T08:49:17Z"
}
```

**`WhitelistView`** :

```json
{
  "id": "01a0a967-f45f-77f6-a7d9-c7622bfc5e2c",
  "hytale_server_id": "01a0a967-cbaa-7746-b9e8-b16daee6d50b",
  "hytale_id": "11111111-1111-7111-8111-111111111111",
  "created_at": "2026-09-16T08:49:17Z",
  "updated_at": "2026-09-16T08:49:17Z"
}
```

**`ModuleView`** :

```json
{
  "id": "01a0a967-cb39-76c1-a30a-4983d3633604",
  "name": "plugin-hytale",
  "description": "plugin de jeu",
  "is_enabled": true,
  "scopes": ["player_sessions:read", "player_sessions:write"],
  "created_at": "2026-09-16T08:49:06Z",
  "updated_at": "2026-09-16T08:49:06Z"
}
```

---

## Public

### GET /health

Vérifie le service et la base.

- **Entrée** : aucune.
- **Sortie 200** : `{ "status": "ok", "database": "up" }`

---

## Administration — `X-Admin-Token`

### POST /api/v1/admin/modules

Enregistre un module. **L'`api_key` et le `hmac_secret` ne sont renvoyés qu'ici, en clair, une seule fois.**

- **Entrée** :

```json
{
  "name": "plugin-hytale",
  "description": "plugin de jeu",
  "scopes": ["player_sessions:read", "player_sessions:write"]
}
```

| Champ | Type | Requis |
|---|---|---|
| `name` | string | oui (unique) |
| `description` | string \| null | non |
| `scopes` | string[] | non (défaut `[]`) |

- **Sortie 201** :

```json
{
  "id": "01a0a967-...",
  "name": "plugin-hytale",
  "api_key": "0ffe27ed...",
  "hmac_secret": "6a69a7c7...",
  "scopes": ["player_sessions:read", "player_sessions:write"]
}
```

- **Erreurs** : 400 (`name` vide), 409 (nom déjà pris).

### GET /api/v1/admin/modules

Liste les modules (sans secrets).

- **Entrée** : aucune.
- **Sortie 200** : `ModuleView[]`.

### GET /api/v1/admin/modules/{id}

Détail d'un module.

- **Entrée** : `id` (UUID, chemin).
- **Sortie 200** : `ModuleView`.
- **Erreurs** : 404.

### POST /api/v1/admin/servers

Enregistre un serveur Hytale.

- **Entrée** :

```json
{ "module_id": "01a0a967-...", "name": "survie", "url": "https://hytale.example/survie" }
```

| Champ | Type | Requis |
|---|---|---|
| `module_id` | UUID | oui |
| `name` | string | oui (unique par module) |
| `url` | string | oui |

- **Sortie 201** : `HytaleServerView`.
- **Erreurs** : 400 (champ vide), 409 (nom déjà pris pour ce module).

### GET /api/v1/admin/servers

Liste paginée des serveurs.

- **Entrée** (query) :

| Param | Type | Défaut | Note |
|---|---|---|---|
| `module_id` | UUID | — | filtre |
| `limit` | int | 50 | borné 1–200 |
| `offset` | int | 0 | ≥ 0 |

- **Sortie 200** : `Page<HytaleServerView>`.

### GET /api/v1/admin/servers/{id}

- **Entrée** : `id` (UUID). **Sortie 200** : `HytaleServerView`. **Erreur** : 404.

### DELETE /api/v1/admin/servers/{id}

Suppression logique (`deleted_at`).

- **Entrée** : `id` (UUID). **Sortie 204** (vide). **Erreur** : 404.

### POST /api/v1/admin/whitelists

Ajoute une entrée de whitelist.

- **Entrée** : `{ "hytale_server_id": "UUID", "hytale_id": "UUID" }`.
- **Sortie 201** : `WhitelistView`. Idempotent : renvoie l'entrée active si elle existe.

### DELETE /api/v1/admin/whitelists/{id}

Suppression logique.

- **Entrée** : `id` (UUID). **Sortie 204** (vide). **Erreur** : 404.

---

## Module — lecture (`X-Api-Key`)

### GET /api/v1/sessions

Liste paginée des sessions. Scope : `player_sessions:read`.

- **Entrée** (query) :

| Param | Type | Défaut | Note |
|---|---|---|---|
| `hytale_server_id` | UUID | — | filtre |
| `hytale_id` | UUID | — | filtre |
| `limit` | int | 50 | borné 1–200 |
| `offset` | int | 0 | ≥ 0 |

- **Sortie 200** : `Page<PlayerSessionView>`.
- **Erreurs** : 401, 403 (scope manquant).

### GET /api/v1/whitelists

Liste paginée de la whitelist. Scope : `whitelist:read`.

- **Entrée** (query) : `hytale_server_id`, `hytale_id`, `limit`, `offset` (idem ci-dessus).
- **Sortie 200** : `Page<WhitelistView>`.
- **Erreurs** : 401, 403.

---

## Module — écriture (`X-Api-Key` + HMAC)

### POST /api/v1/whitelists

Ajoute une entrée. Scopes : `whitelist:write`. Le serveur doit appartenir au module.

- **Entrée** : `{ "hytale_server_id": "UUID", "hytale_id": "UUID" }`.
- **Sortie 201** : `WhitelistView`. Idempotent.
- **Erreurs** : 401, 403 (scope manquant ou serveur non possédé).

### DELETE /api/v1/whitelists/{id}

Suppression logique. Scope : `whitelist:write`. L'entrée doit dépendre d'un serveur du module.

- **Entrée** : `id` (UUID, chemin).
- **Sortie 204** (vide).
- **Erreurs** : 401, 403, 404.

### POST /api/v1/webhook

Ingestion d'événements. Scope : `player_sessions:write`. Le serveur doit appartenir au module.

- **Entrée** — variantes (discriminant `event_type`) :

```json
{ "event_type": "session_started", "hytale_server_id": "UUID", "hytale_id": "UUID", "joined_at": "2026-09-16T08:00:00Z" }
```

```json
{ "event_type": "session_ended", "hytale_server_id": "UUID", "hytale_id": "UUID" }
```

| Champ | Type | Requis |
|---|---|---|
| `event_type` | `"session_started"` \| `"session_ended"` | oui |
| `hytale_server_id` | UUID | oui |
| `hytale_id` | UUID | oui |
| `joined_at` | RFC 3339 | non (défaut `now()`), `session_started` uniquement |

- **Sortie 202** (`session_started`) :

```json
{ "id": "01a0a967-...", "status": "accepted" }
```

- **Sortie 202** (`session_ended`) :

```json
{ "hytale_id": "11111111-...", "status": "closed" }
```

`status` vaut `"no_open_session"` si aucune session ouverte n'a été trouvée.

- **Erreurs** : 400 (`event_type` inconnu/champ manquant), 401, 403.

### Exemple de signature

```bash
TS=$(date +%s)
BODY='{"event_type":"session_ended","hytale_server_id":"UUID","hytale_id":"UUID"}'
SIG=$(printf '%s.%s' "$TS" "$BODY" | openssl dgst -sha256 -hmac "$HMAC_SECRET" -hex | awk '{print $2}')

curl -X POST http://localhost:3000/api/v1/webhook \
  -H "x-api-key: $API_KEY" \
  -H "x-timestamp: $TS" \
  -H "x-signature: $SIG" \
  -H 'content-type: application/json' \
  -d "$BODY"
```
