# ADR-0006: Service layer, no repository trait

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

As the number of consumers grew, the HTTP handlers had accumulated business
logic (ownership checks, idempotence, scope enforcement). We wanted the business
rules to be independent of the transport and testable without a server.

We also considered abstracting data access behind a `Repository` trait with one
implementation per backend, a common layering pattern.

## Decision

Each domain is split into four flat files:

- `model.rs` — `FromRow` rows, `Serialize` views, `Deserialize` requests;
- `repo.rs` — pure SQL, no business rules;
- `service.rs` — business rules, ownership, idempotence, validations;
- `handler.rs` — HTTP extraction, scope checks, calls `service`, serialization.

`service` must never depend on `http`. `handler` must contain no SQL.

We do **not** introduce a `Repository` trait. There is a single backend
(PostgreSQL), and `#[sqlx::test]` already gives fast, isolated database tests.
A trait would add `async-trait`/generics/`Box<dyn>` complexity for no current
benefit.

## Consequences

- Business logic is unit-testable without spinning up HTTP.
- Adding an endpoint is a local change inside one domain folder.
- Introducing a second backend or a mock-driven testing strategy is the trigger
  to revisit this ADR and add the trait.
