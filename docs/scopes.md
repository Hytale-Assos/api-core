# Scopes — reference

Last updated: 18 September 2026

A **scope** is a named permission granted to a module (see `module_scopes` in
`docs/schema.md`). It replaced the old `kind` field and allows fine-grained
authorization of a module according to the service it represents (website,
Hytale plugin, Discord bot, backoffice...).

## Format

```
<resource>:<action>
```

- `resource`: the business table/entity concerned (e.g. `player_sessions`).
- `action`: `read` or `write`.

Storage is free-form `TEXT` in the database (extensible without migration). The
reference constants live in `src/shared/scopes.rs`.

## Current scopes

| Scope | Constant | Access granted | Endpoints concerned |
|---|---|---|---|
| `player_sessions:read` | `PLAYER_SESSIONS_READ` | Read sessions | `GET /api/v1/sessions`, `GET /api/v1/players/{hytale_id}/sessions` |
| `player_sessions:write` | `PLAYER_SESSIONS_WRITE` | Create/end sessions | `POST /api/v1/webhook` (events `session_started`, `session_ended`) |
| `whitelist:read` | `WHITELIST_READ` | Read the whitelist | `GET /api/v1/whitelists`, `GET /api/v1/players/{hytale_id}/whitelists` |
| `whitelist:write` | `WHITELIST_WRITE` | Add/remove entries (self-service, no ownership, ADR-0016) | `POST /api/v1/whitelists`, `DELETE /api/v1/whitelists/{id}` |
| `hytale_servers:read` | `HYTALE_SERVERS_READ` | List/read **all** servers | `GET /api/v1/servers`, `GET /api/v1/servers/{id}` |
| `hytale_servers:write` | `HYTALE_SERVERS_WRITE` | Create/delete a server | `POST /api/v1/servers`, `DELETE /api/v1/servers/{id}` |
| `modules:read` | `MODULES_READ` | List/read modules | `GET /api/v1/modules`, `GET /api/v1/modules/{id}` |
| `modules:write` | `MODULES_WRITE` | Register a module | `POST /api/v1/modules` |
| `admin` | `ADMIN` | Bypasses ownership checks | backoffice |

## Ownership vs visibility

There are no more `/admin/*` endpoints protected by a separate token: the
backoffice is a **module** holding the `admin` scope and the `*:read` /
`*:write` scopes.

- **Writes**: a module may only create/delete its own servers
  (`assert_owned`). The `admin` scope (backoffice) bypasses this restriction.
  **Whitelist entries are exempt** (`whitelist:write` is self-service, see
  ADR-0016): a member whitelists themselves on a server they do not own.
- **Reads**: `hytale_servers:read` grants access to **all** servers. Reading is
  not risky, only modifications are restricted. The `module_id` filter lets a
  caller voluntarily narrow the list.

## Routes not concerned by scopes

| Route | Protection |
|---|---|
| `GET /health` | public |

All other routes require `X-Api-Key` (reads) or `X-Api-Key` + HMAC signature
(writes).

## Attribution

- Scopes are provided at module creation:
  `POST /api/v1/modules` with `"scopes": ["player_sessions:read", ...]`.
- The `admin` module is seeded at startup if `SEED_ADMIN_API_KEY` and
  `SEED_ADMIN_HMAC_SECRET` are set.
- The `module_auth` middleware (`src/http/middleware.rs`) loads scopes into the
  `Principal` (`src/shared/identity.rs`); each handler checks them through
  `principal.require(...)` and returns `403 Forbidden` otherwise.

## Adding a scope

1. Add the constant in `src/shared/scopes.rs`.
2. Check it in the relevant handler (`principal.require(...)`).
3. Document it in the table above.
4. No SQL migration needed (`TEXT` storage).
