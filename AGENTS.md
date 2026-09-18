# AGENTS.md

Guidance for AI agents (and humans) working in this repository.

## Project in one line

`api-core` is the hub of the Hytale ecosystem: a REST API that centralizes
Hytale servers, player sessions, and whitelists for all consumers (plugin,
Laravel website, Discord bot, orchestration). It is a Rust/Axum + PostgreSQL
modular monolith.

## Before you change anything — check the ADRs

When you face a decision, **first check whether an ADR already documents it**
in [`docs/decisions/`](docs/decisions/README.md). ADRs are the source of truth
for durable architectural choices (auth, encryption, soft delete, ownership,
layering, transport...).

- If an ADR covers the decision, follow it. Do not silently deviate.
- If you believe an ADR is wrong, do not edit it: propose a new ADR that
  supersedes it, and get confirmation.
- If the decision is significant and no ADR covers it, create a new ADR using
  the format in `docs/decisions/README.md`, then implement.

Significant means: it shapes module boundaries, affects security/ownership/
data integrity, changes the public API contract, or is expensive to reverse.

## Architecture rules (non-negotiable)

- **Hub, not a client backend.** Consumers call the core; the core calls no
  consumer. Never put client-specific logic (OAuth, Discord commands, web
  pages) in the core.
- **Everything is a module.** There is one authentication model: API key for
  reads, API key + HMAC for writes. No admin token, no `/admin/*` routes.
- **No user accounts in the core.** Only modules and `hytale_id` values.
- **`hytale_id` is a data filter, not an identity.** Authorization comes from
  the `Principal` and its scopes only.
- **Reads are global, writes are ownership-restricted.** `assert_owned` is the
  single ownership gate; `admin` bypasses it.
- **No outbound network calls in an HTTP request path.** Long effects go
  through the outbox/worker (see ADR-0010).

## Domain file layout

Each domain lives in `src/domains/<domain>/` with four flat files:

| File | Contains | Must not contain |
|---|---|---|
| `model.rs` | rows, views, request/query DTOs | business logic |
| `repo.rs` | pure SQL | business rules, `assert_*` |
| `service.rs` | business rules, ownership, idempotence | Axum/HTTP types |
| `handler.rs` | extraction, scope checks, serialization | SQL |

`service` may depend on another domain's `service`, never on `http`. HTTP
middleware and extractors live in `src/http/`; ingestion adapters in
`src/ingest/`; cross-cutting helpers in `src/shared/`.

Do not add a `Repository` trait (ADR-0006). `sqlx::test` covers DB testing.

## Data rules

- Soft delete only: filter `deleted_at IS NULL` in every read, use partial
  unique indexes for reusable names (ADR-0007).
- UUID v7 primary keys generated in the app (ADR-0008).
- Encrypt secrets (AES-256-GCM), hash searchable values (HMAC) (ADR-0009).
  Never add a clear-text secret column.

## Workflow

- Language: code (including comments) and docs are **English**; commits are
  **English**, imperative, following
  [`docs/git-conventions.md`](docs/git-conventions.md).
- User-facing error messages are centralized in `src/error.rs` (`messages`
  module). Never inline a literal message at the call site.
- Before finishing any change, run and make pass:

  ```bash
  cargo fmt
  cargo clippy --all-targets
  cargo test          # needs DATABASE_URL pointing to a Postgres
  ```

- One logical change per commit. Never commit secrets (`.env`).
- Do **not** commit unless explicitly asked.
- Keep docs in sync in the same change: `docs/api.md`, `docs/scopes.md`,
  `docs/schema.md`, and the ADRs when a decision is involved.

## Definition of done

1. Behavior implemented in the correct domain and layer (service, not handler).
2. Scopes enforced via `principal.require(...)`.
3. Ownership enforced via `servers::service::assert_owned` where relevant.
4. Unit + HTTP integration tests added (`src/tests/` mounts the real router).
5. `cargo fmt`, `clippy`, and `test` pass.
6. Docs and ADRs updated if the contract or a decision changed.
