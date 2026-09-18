# API — endpoint reference

Last updated: 18 September 2026

Base URL: `http://<host>:3000`. JSON body (UTF-8).

## Authentication

There is a single authentication model: **every consumer is a module** (Hytale
plugin, Laravel site, Discord bot, backoffice). Two modes depending on the HTTP
method:

| Method | Required headers |
|---|---|
| `GET` | `X-Api-Key: <api_key>` |
| `POST` / `DELETE` | `X-Api-Key`, `X-Timestamp` (Unix seconds), `X-Signature` |

`X-Signature` = hex HMAC-SHA256 of `"{X-Timestamp}.{raw body}"`, computed with
the module's `hmac_secret`. Timestamp tolerance: `HMAC_TOLERANCE_SECS`
(default 300). Signatures and scopes: see `docs/scopes.md`.

> `GET /health` is public. There are no more admin endpoints protected by a
> token: the backoffice is a module holding the `admin` scope.

## Error format

Every error returns JSON:

```json
{ "error": "readable message" }
```

| Status | Meaning |
|---|---|
| 400 | invalid body/parameter |
| 401 | missing authentication, unknown key, invalid signature or expired timestamp |
| 403 | disabled module, missing scope, or resource outside its scope |
| 404 | resource not found |
| 409 | conflict (name already used) |
| 500 | internal error (details never exposed) |

## Shared types

**`Page<T>`** (list responses):

```json
{ "items": [], "total": 0, "limit": 50, "offset": 0 }
```

**`HytaleServerView`**:

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

**`PlayerSessionView`**:

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

**`WhitelistView`**:

```json
{
  "id": "01a0a967-f45f-77f6-a7d9-c7622bfc5e2c",
  "hytale_server_id": "01a0a967-cbaa-7746-b9e8-b16daee6d50b",
  "hytale_id": "11111111-1111-7111-8111-111111111111",
  "created_at": "2026-09-16T08:49:17Z",
  "updated_at": "2026-09-16T08:49:17Z"
}
```

**`ModuleView`**:

```json
{
  "id": "01a0a967-cb39-76c1-a30a-4983d3633604",
  "name": "plugin-hytale",
  "description": "game plugin",
  "is_enabled": true,
  "scopes": ["player_sessions:read", "player_sessions:write"],
  "created_at": "2026-09-16T08:49:06Z",
  "updated_at": "2026-09-16T08:49:06Z"
}
```

> `hytale_id` is always a **data filter**, never an identity: authorization is
> carried by the authenticated module and its scopes.

---

## Public

### GET /health

Checks the service and the database.

- **Input**: none.
- **Output 200**: `{ "status": "ok", "database": "up" }`

---

## Modules — registry

### POST /api/v1/modules

Registers a module. Scope: `modules:write`. **The `api_key` and `hmac_secret`
are returned only here, in the clear, once.**

- **Input**:

```json
{
  "name": "plugin-hytale",
  "description": "game plugin",
  "scopes": ["player_sessions:read", "player_sessions:write"]
}
```

| Field | Type | Required |
|---|---|---|
| `name` | string | yes (unique) |
| `description` | string \| null | no |
| `scopes` | string[] | no (default `[]`) |

- **Output 201**:

```json
{
  "id": "01a0a967-...",
  "name": "plugin-hytale",
  "api_key": "0ffe27ed...",
  "hmac_secret": "6a69a7c7...",
  "scopes": ["player_sessions:read", "player_sessions:write"]
}
```

- **Errors**: 400 (empty `name`), 403 (missing scope), 409 (name already taken).

### GET /api/v1/modules

Lists modules (without secrets). Scope: `modules:read`.

- **Output 200**: `ModuleView[]`.

### GET /api/v1/modules/{id}

Details of a module. Scope: `modules:read`.

- **Input**: `id` (UUID, path).
- **Output 200**: `ModuleView`. **Errors**: 403, 404.

---

## Hytale servers

### POST /api/v1/servers

Registers a server. Scope: `hytale_servers:write`. A non-admin module may only
create for itself (`module_id` = its own id); admin may create for any module.

- **Input**:

```json
{ "module_id": "01a0a967-...", "name": "survie", "url": "https://hytale.example/survie" }
```

| Field | Type | Required |
|---|---|---|
| `module_id` | UUID | yes |
| `name` | string | yes (unique per module) |
| `url` | string | yes |

- **Output 201**: `HytaleServerView`.
- **Errors**: 400 (empty field), 403, 409 (name already taken for this module).

### GET /api/v1/servers

Paginated list of servers. Scope: `hytale_servers:read`. Reads are **global**:
any module with the scope sees all servers.

- **Input** (query):

| Param | Type | Default | Note |
|---|---|---|---|
| `module_id` | UUID | — | optional filter by owning module |
| `limit` | int | 50 | clamped 1–200 |
| `offset` | int | 0 | ≥ 0 |

- **Output 200**: `Page<HytaleServerView>`. **Errors**: 403.

### GET /api/v1/servers/{id}

Scope: `hytale_servers:read`. Global read.

- **Output 200**: `HytaleServerView`. **Errors**: 403, 404.

### DELETE /api/v1/servers/{id}

Soft delete (`deleted_at`). Scope: `hytale_servers:write`. Ownership required
(except admin).

- **Output 204** (empty). **Errors**: 403, 404.

---

## Player sessions — reads

### GET /api/v1/sessions

Paginated list of sessions. Scope: `player_sessions:read`.

- **Input** (query):

| Param | Type | Default | Note |
|---|---|---|---|
| `hytale_server_id` | UUID | — | filter |
| `hytale_id` | UUID | — | filter |
| `limit` | int | 50 | clamped 1–200 |
| `offset` | int | 0 | ≥ 0 |

- **Output 200**: `Page<PlayerSessionView>`. **Errors**: 401, 403.

### GET /api/v1/players/{hytale_id}/sessions

A player's sessions, optionally filtered by server. Scope:
`player_sessions:read`. Used by the Laravel site and the Discord bot.

- **Input** (path + query): `hytale_id` (path), then `hytale_server_id`,
  `limit`, `offset`.
- **Output 200**: `Page<PlayerSessionView>`. **Errors**: 401, 403.

---

## Whitelist

### GET /api/v1/whitelists

Paginated list of the whitelist. Scope: `whitelist:read`.

- **Input** (query): `hytale_server_id`, `hytale_id`, `limit`, `offset`.
- **Output 200**: `Page<WhitelistView>`. **Errors**: 401, 403.

### GET /api/v1/players/{hytale_id}/whitelists

A player's whitelist, optionally filtered by server. Scope: `whitelist:read`.

- **Output 200**: `Page<WhitelistView>`. **Errors**: 401, 403.

### POST /api/v1/whitelists

Adds an entry. Scope: `whitelist:write`. The server must belong to the module
(except admin).

- **Input**: `{ "hytale_server_id": "UUID", "hytale_id": "UUID" }`.
- **Output 201**: `WhitelistView`. Idempotent.
- **Errors**: 401, 403 (missing scope or server not owned).

### DELETE /api/v1/whitelists/{id}

Soft delete. Scope: `whitelist:write`. The entry must belong to a server of the
module (except admin).

- **Input**: `id` (UUID, path).
- **Output 204** (empty). **Errors**: 401, 403, 404.

---

## Ingestion — webhook

### POST /api/v1/webhook

Event ingestion. Scope: `player_sessions:write`. The server must belong to the
module (except admin).

- **Input** — variants (discriminant `event_type`):

```json
{ "event_type": "session_started", "hytale_server_id": "UUID", "hytale_id": "UUID", "joined_at": "2026-09-16T08:00:00Z" }
```

```json
{ "event_type": "session_ended", "hytale_server_id": "UUID", "hytale_id": "UUID" }
```

| Field | Type | Required |
|---|---|---|
| `event_type` | `"session_started"` \| `"session_ended"` | yes |
| `hytale_server_id` | UUID | yes |
| `hytale_id` | UUID | yes |
| `joined_at` | RFC 3339 | no (default `now()`), `session_started` only |

- **Output 202** (`session_started`):

```json
{ "id": "01a0a967-...", "status": "accepted" }
```

- **Output 202** (`session_ended`):

```json
{ "hytale_id": "11111111-...", "status": "closed" }
```

`status` is `"no_open_session"` if no open session was found.

- **Errors**: 400 (unknown `event_type`/missing field), 401, 403.

### Signature example

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

### Bootstrapping the first module (backoffice)

At startup, if `SEED_ADMIN_API_KEY` and `SEED_ADMIN_HMAC_SECRET` are set and no
module named `admin` exists, the core creates it with all scopes. Then create
the other modules with a signed `curl`:

```bash
BODY='{"name":"siteweb","scopes":["hytale_servers:read","player_sessions:read","whitelist:read","whitelist:write"]}'
TS=$(date +%s)
SIG=$(printf '%s.%s' "$TS" "$BODY" | openssl dgst -sha256 -hmac "$SEED_ADMIN_HMAC_SECRET" -hex | awk '{print $2}')

curl -X POST http://localhost:3000/api/v1/modules \
  -H "x-api-key: $SEED_ADMIN_API_KEY" \
  -H "x-timestamp: $TS" \
  -H "x-signature: $SIG" \
  -H 'content-type: application/json' \
  -d "$BODY"
```
