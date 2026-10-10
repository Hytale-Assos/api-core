# Database schema — validated decisions

Last updated: 18 September 2026

This document freezes the modeling decisions made during the diagram review,
before writing migration `0002`. It supersedes the choices of the initial
diagram (`Blank diagram - Page 1.csv`).

## 1. Cross-cutting principles

| Topic | Decision | Reason |
|---|---|---|
| Primary key | `UUID v7` on **every** table | Time-ordered → good index locality (no fragmentation like v4), still unguessable and uniform on the Rust side (`sqlx::FromRow`). Requires the `v7` feature of the `uuid` crate. Legacy v4 `modules.id` remain valid (a UUID is a UUID). |
| Timestamp | `TIMESTAMPTZ` everywhere | Timezone-independent, consistent with `0001_init.sql`. |
| Lifecycle | `is_enabled BOOLEAN NOT NULL DEFAULT TRUE` **and** `deleted_at TIMESTAMPTZ NULL` | Two distinct states: reversible disabling (`is_enabled=false`) vs removal (`deleted_at`). Not redundant. |
| Deletion | Soft delete only | Preserves history and FK stability. Requires filtering `deleted_at IS NULL` in every read and partial indexes for uniqueness constraints. |
| `updated_at` | `set_updated_at()` trigger on each table | Sets `NEW.updated_at = now()` on every `UPDATE`, automatically and reliably (no dependency on application code). |
| Secrets | `api_key_hash` (deterministic HMAC-SHA256) + `hmac_secret` (AES-256-GCM) | Unchanged: see `src/crypto.rs`. Application-level encryption, the database never sees the master key. |

## 2. Encryption: what is and is not encrypted

Encryption is **application-level**: it happens in the core (`src/crypto.rs`)
before insertion. The database never holds the master key (`ENCRYPTION_KEY`),
so a PostgreSQL dump alone cannot read the secrets.

### Encrypted (AES-256-GCM)

Stored format: `TEXT` = `base64(nonce || ciphertext || tag)`.

| Table | Column | Why |
|---|---|---|
| `modules` | `hmac_secret` | HMAC signing secret: must be **decrypted** by the core to verify signatures. |

### Deterministically hashed (HMAC-SHA256, irreversible)

| Table | Column | Why |
|---|---|---|
| `modules` | `api_key_hash` | Must stay **searchable** (authentication lookup index) without ever storing the API key in the clear. Deterministic = same key → same hash. |

### In the clear (not encrypted)

| Table(s) | Columns | Why |
|---|---|---|
| `modules` | `id`, `name`, `description`, `is_enabled`, `deleted_at`, `created_at`, `updated_at` | Non-sensitive data, needed for filters/joins/admin display. |
| `module_scopes` | `module_id`, `scope` | Used for authorization and joins. |
| `hytale_servers` | all | `name`, `url` are used for display and routing. |
| `player_sessions` | `id`, `hytale_server_id`, `hytale_id`, `joined_at`, `ended_at`, ... | Requires indexes and queries by `hytale_id` and by date. |
| `whitelists` | `id`, `hytale_server_id`, `hytale_id`, ... | Uniqueness constraint and lookups by `hytale_id`. |

> Rule: a column is encrypted only if it must stay **secret while remaining
> recoverable**. If it must be **searchable**, it is hashed. Otherwise it stays
> in the clear.

## 3. Target schema

### `modules`
Module registered with the core (website, Hytale plugin, Discord bot...).

| Column | Type | Null | Note |
|---|---|---|---|
| `id` | UUID PK | no | v7 |
| `name` | TEXT | no | UNIQUE (global) |
| `description` | TEXT | **yes** | |
| `api_key_hash` | TEXT | no | deterministic HMAC, UNIQUE, lookup index |
| `hmac_secret` | TEXT | no | AES-256-GCM encrypted |
| `is_enabled` | BOOLEAN | no | default TRUE (replaces `active`) |
| `deleted_at` | TIMESTAMPTZ | yes | |
| `created_at` | TIMESTAMPTZ | no | default `now()` |
| `updated_at` | TIMESTAMPTZ | no | default `now()` |

> `kind` is **removed** in favor of scopes (table `module_scopes`).

### `module_scopes`
Scopes granted to a module (replaces `kind`).

| Column | Type | Null | Note |
|---|---|---|---|
| `module_id` | UUID FK → `modules(id)` | no | `ON DELETE CASCADE` |
| `scope` | TEXT | no | e.g. `player_sessions:write`, `whitelist:read` |
| PK | composite `(module_id, scope)` | | |

> Authorization by service type is done through scopes. The webhook checks that
> the calling module holds the scope matching the targeted business table.

### `hytale_servers`
Hytale server managed by a module.

| Column | Type | Null | Note |
|---|---|---|---|
| `id` | UUID PK | no | v7 |
| `module_id` | UUID FK → `modules(id)` | no | |
| `name` | TEXT | no | |
| `url` | TEXT | no | |
|  `agent_url` | TEXT | yes | agent control-plane base URL (ADR-0016) |
|  `agent_token` | TEXT | yes | AES-256-GCM encrypted agent token (ADR-0009) |
|  `agent_registered_at` | TIMESTAMPTZ | yes | last agent registration |
| `deleted_at` | TIMESTAMPTZ | yes | |
| `created_at` | TIMESTAMPTZ | no | default `now()` |
| `updated_at` | TIMESTAMPTZ | no | default `now()` |

### `whitelist_queue`
Pending management commands delivered to registered servers (ADR-0016). It is a
delivery **queue**, distinct from `whitelists` (which holds the desired state):
a command must survive the removal of the whitelist entry it refers to.

| Column | Type | Null | Note |
|---|---|---|---|
| `id` | UUID PK | no | v7 |
| `hytale_server_id` | UUID FK → `hytale_servers(id)` | no | |
| `hytale_id` | UUID | no | |
| `action` | TEXT | no | `add` or `remove` |
| `status` | TEXT | no | `pending`, `sent`, `failed` |
| `attempts` | INTEGER | no | default 0 |
| `last_error` | TEXT | yes | last delivery error |
| `next_attempt_at` | TIMESTAMPTZ | no | backoff schedule |
| `created_at` / `updated_at` | TIMESTAMPTZ | no | |

> `whitelist_queue` has no FK to `whitelists` on purpose: a `remove` command
> must outlive the soft-deleted entry. Partial unique index on
> `(hytale_server_id, hytale_id, action)` where `status = 'pending'` keeps
> repeated toggles from piling up duplicates.

### `player_sessions`
Player connection session to a server.

| Column | Type | Null | Note |
|---|---|---|---|
| `id` | UUID PK | no | v7 |
| `hytale_server_id` | UUID FK → `hytale_servers(id)` | no | |
| `hytale_id` | UUID | no | external identity provided by plugin/site (no `players` table) |
| `joined_at` | TIMESTAMPTZ | no | effective entry into the world |
| `ended_at` | TIMESTAMPTZ | yes | end of session; `NULL` = session in progress |
| `deleted_at` | TIMESTAMPTZ | yes | |
| `created_at` | TIMESTAMPTZ | no | default `now()` |
| `updated_at` | TIMESTAMPTZ | no | default `now()` |

> **Decision: the plugin only sends a request once the player has entered the
> world.** We therefore model only `joined_at` (non-nullable) and `ended_at`.
> No `started_at`: no source provides the connection-attempt event. If tracking
> attempts becomes necessary, add the plugin hook **and** the column by
> migration at that point.

### `whitelists`
Whitelist entry of a player for a server.
**Decision: the whitelist is attached to a Hytale server** (one whitelist per
server), to manage multiple servers independently.

| Column | Type | Null | Note |
|---|---|---|---|
| `id` | UUID PK | no | v7 |
| `hytale_server_id` | UUID FK → `hytale_servers(id)` | no | |
| `hytale_id` | UUID | no | |
| `deleted_at` | TIMESTAMPTZ | yes | |
| `created_at` | TIMESTAMPTZ | no | default `now()` |
| `updated_at` | TIMESTAMPTZ | no | default `now()` |

## 4. Cardinalities (Crow's foot)

```
modules          ||--o{ module_scopes
modules          ||--o{ hytale_servers
hytale_servers   ||--o{ player_sessions
hytale_servers   ||--o{ whitelists
```

- `module_scopes.module_id`: `NOT NULL`.
- `hytale_servers.module_id`: `NOT NULL`.
- `player_sessions.hytale_server_id`: `NOT NULL`.
- `whitelists.hytale_server_id`: `NOT NULL`.

## 5. Indexes and constraints

| Table | Constraint / index | Detail |
|---|---|---|
| `modules` | `UNIQUE (name)` | already present |
| `modules` | `UNIQUE (api_key_hash)` | already present |
| `module_scopes` | PK `(module_id, scope)` | |
| `hytale_servers` | `UNIQUE (module_id, name) WHERE deleted_at IS NULL` | partial (soft delete) |
| `hytale_servers` | `INDEX (module_id)` | |
| `player_sessions` | `INDEX (hytale_server_id, joined_at DESC)` | session list |
| `player_sessions` | `INDEX (hytale_id)` | "player's sessions" query (site) |
| `player_sessions` | `INDEX (ended_at) WHERE ended_at IS NULL` | sessions in progress |
| `whitelists` | `UNIQUE (hytale_server_id, hytale_id) WHERE deleted_at IS NULL` | partial, allows re-whitelist after deletion |
| `whitelists` | `INDEX (hytale_id)` | "am I whitelisted" check (site) |

## 6. Removal of the `events` table

- The `events` table (raw log) is **removed** from the target model.
- The webhook writes **directly** to the business table, restricted by the scope
  of the module authenticated via HMAC (`src/http/middleware.rs`).

**Accepted trade-offs:**
- no more append-only log (audit trail) or idempotence by `event_id`;
- each new event type requires a migration;
- the core becomes coupled to the Hytale domain (`event_type` → table routing).

> If an audit/replay need reappears, plan a lightweight `event_log` table
> (append-only, without business mapping) rather than reintroducing projection
> logic.

## 7. Decision: no `players` table

Player identity (`hytale_id`) stays **denormalized** in `player_sessions` and
`whitelists`.

- `hytale_id` is a UUID provided by Hytale.
- Plugin side: passed in requests to know who does what.
- Website side: obtained through "Sign in with Hytale" (OAuth handled in the
  website module, **not** in the core), then direct queries by `hytale_id`.

Reason invoked: avoid an extra SQL round-trip (parse → lookup player → insert)
on every connection.

**Accepted downside:** if player-specific data (display_name, first/last seen,
preferences...) ever needs to be persisted, a `players` table will have to be
reintroduced and the `hytale_id` migrated to a `player_id`.

## 8. Transport: REST (not gRPC)

Question raised: replace REST webhooks with gRPC (streaming) for event
ingestion.

**Decision: stay on a REST API** (HTTP webhooks + read endpoints).

Reasons:

- Load is not a problem: Axum/Tokio easily absorbs 20+ simultaneous
  connections. The real bottleneck would be the SQLx pool (`max_connections`),
  not the HTTP transport.
- Simple integration for third-party plugins (a signed HTTP POST is trivial to
  emit, including from a game plugin), versus a gRPC/HTTP-2/TLS client that is
  heavier to embed.
- Easy debugging and operations (`curl`, HTTP logs), unlike gRPC which requires
  `grpcurl` and an HTTP/2-compatible load balancer.

**Accepted trade-offs:** no real-time presence semantics (a long-lived
connection), no typed protobuf contract, implicit ack through the 202.

> gRPC remains a documented option for later (typed data plane + stream +
> per-message ack), to be enabled only if a true real-time presence/audit need
> appears.

## 9. Open points

1. **Retention / purge** (GDPR) of soft-deleted rows (volume).

## 10. Impact on the existing code

| File | Change |
|---|---|
| `migrations/0001_init.sql` | **Rewritten** to the target schema (nothing committed, no `0002`). `events` removed, `kind` → `module_scopes`, `active` → `is_enabled`, UUID v7, `updated_at` triggers, partial indexes. |
| `src/domains/<domain>/model.rs` | `FromRow` rows, `Serialize` views, `Deserialize` requests, split by domain. |
| `src/domains/<domain>/repo.rs` | SQL queries per business table; `deleted_at IS NULL` filters. |
| `src/domains/<domain>/service.rs` | Business rules, ownership and idempotence, outside the HTTP transport. |
| `src/ingest/webhook.rs` | Scope-based routing to the relevant domain service. |
| `src/http/middleware.rs` | API key/HMAC verification then `Principal` injection. |
| `src/crypto.rs` | Unchanged (AES-256-GCM + deterministic HMAC). |
