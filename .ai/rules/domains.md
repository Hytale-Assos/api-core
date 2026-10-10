---
paths:
  - 'src/domains/**'
---

# Domains

Each domain lives in `src/domains/<domain>/` with four flat files. Keep the
layers clean: this is the core rule of the codebase (ADR-0001, ADR-0006).

| File | Contains | Must not contain |
| --- | --- | --- |
| `model.rs` | rows, views, request/query DTOs | business logic |
| `repo.rs` | pure SQL | business rules, `assert_*` |
| `service.rs` | business rules, ownership, idempotence | Axum/HTTP types |
| `handler.rs` | extraction, scope checks, serialization | SQL |

## Layering

- `service` may depend on another domain's `service`, never on `http`.
- `repo` may be called only from its own domain's `service` (and
  `src/tests/`). Handlers never call repos directly.
- HTTP middleware and extractors live in `src/http/`; ingestion adapters in
  `src/ingest/`; cross-cutting helpers in `src/shared/`.
- Do not add a `Repository` trait (ADR-0006). Functions in `repo.rs` are
  plain `async fn` taking the `PgPool` (or a transaction) directly.
- `sqlx::test` covers DB testing; shared fixtures live in `src/testutil.rs`
  (ADR-0015).

## Authorization

- Scopes are enforced via `principal.require(...)` in the handler, before
  touching the service.
- Ownership is enforced via `servers::service::assert_owned` in the service,
  wherever the write targets an owned resource (ADR-0005). `admin` scope
  bypasses it; never hand-roll a second ownership gate.
- `hytale_id` is a data filter, never an identity (ADR-0004): filter by it in
  SQL, never authorize by it.

## Reads and writes

- Every read filters `deleted_at IS NULL` (ADR-0007).
- Reads are global, writes are ownership-restricted — do not add an
  ownership check to a read path, and never skip one on a write path.
- No outbound network call in an HTTP request path: enqueue in the
  outbox/queue instead (ADR-0010, ADR-0016).

## Errors and messages

- User-facing error messages come from the centralized `messages` module in
  `src/error.rs`. Never inline a literal message at the call site.
- Keep logs structured through `src/shared/logs.rs` (ADR-0013).
