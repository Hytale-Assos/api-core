# Architecture — api-core

Last updated: 18 September 2026

`api-core` is the **hub** connecting every service of the Hytale ecosystem.
Consumers never talk to each other directly: they all go through the core.

```
Hytale plugin  ─┐
Laravel site    ─┼──►  api-core (hub)  ──►  PostgreSQL
Discord bot    ─┘         │
                          └──► orchestration (future) ──► Hytale servers
```

## Founding rules

1. **One consumer = one module.** The plugin, the Laravel site, the Discord
   bot, the backoffice and (later) orchestration are all registered as modules
   authenticated by API key + HMAC. There is **a single authentication model**:
   no admin endpoints protected by a separate token.
2. **The core only knows modules and `hytale_id` values.** Never a user
   account. Account creation, OAuth and the account ↔ `hytale_id` mapping live
   in the Laravel site (and the Discord user ↔ `hytale_id` mapping in the bot),
   not in the core.
3. **`hytale_id` is a data filter, not an identity.** Authorization relies only
   on the authenticated `Principal` (module + scopes).
4. **Ownership vs visibility.** Writes on a server are restricted to its owning
   module (`hytale_servers.module_id`). Reads are global for any module holding
   the matching `:read` scope: reading is not risky, only modifications are
   restricted. The `admin` scope bypasses the ownership check (backoffice).
5. **No outbound network calls in an HTTP request path.** Long effects
   (orchestration) will go through an `outbox`/`jobs` table and a worker.

## Code structure

```
src/
├── main.rs                # bootstrap: config, pool, migrations, seed, serve
├── config.rs              # env -> Config
├── error.rs               # AppError + IntoResponse
├── state.rs               # AppState { db, crypto, config }
├── crypto.rs              # AES-256-GCM + deterministic HMAC
├── testutil.rs            # test helpers (#[cfg(test)])
├── http/                  # transport only
│   ├── mod.rs
│   ├── router.rs          # route + layer assembly
│   ├── middleware.rs      # module_auth (API key + HMAC)
│   ├── extractors.rs      # Principal, ...
│   └── health.rs          # GET /health
├── shared/                # cross-cutting utilities
│   ├── mod.rs
│   ├── hmac.rs            # request signature verification
│   ├── identity.rs        # Principal (module + scopes)
│   ├── logs.rs            # centralized log messages
│   ├── scopes.rs          # scope constants
│   └── pagination.rs      # Page<T>, pagination
├── domains/
│   ├── modules/           # module registry + management
│   ├── servers/           # Hytale servers
│   ├── sessions/          # player sessions
│   ├── whitelists/        # player whitelist
│   └── orchestration/     # (future) server updates
└── ingest/
    ├── mod.rs
    └── webhook.rs         # WebhookEvent -> sessions::service
```

Every domain always follows the same flat layout:

| File | Role |
|---|---|
| `model.rs` | `FromRow` rows, `Serialize` views, `Deserialize` requests |
| `repo.rs` | pure SQL, no business rule |
| `service.rs` | business rules, ownership, idempotence, validations |
| `handler.rs` | HTTP extraction, scope check, `service` call |

## Boundaries

- `service` **never** depends on `http`. It is testable without a server.
- `handler` contains **no SQL** and stays thin (~10 lines).
- `repo` contains **no rule** (no `assert_*`).
- `http` is the only place that speaks Axum.
- Repository traits are not introduced while there is a single backend
  (`#[sqlx::test]` covers DB testability).

## Request flow

```
Laravel / bot / plugin
        │  X-Api-Key (+ HMAC on writes)
        ▼
http::middleware module_auth ──► Principal (module + scopes)
        ▼
domains::<domain>::handler
        │  require(scope), extract hytale_id / filters
        ▼
domains::<domain>::service ──► other service (ownership)
        ▼
domains::<domain>::repo ──► PostgreSQL
```

## Bootstrap (seed)

Creating a module requires being a module: there must be a starting point. At
startup, if `SEED_ADMIN_API_KEY` and `SEED_ADMIN_HMAC_SECRET` are set and no
module named `admin` exists, the core creates it with all scopes. The seed is
idempotent. Manage the other modules afterwards with `curl`.
