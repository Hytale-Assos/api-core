# ADR-0015: Shared test fixtures and helpers

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

Test code repeated the same literals and setup blocks in many places. Service
tests each defined their own `seed_principal`, and every test needing a server
repeated the same `servers::create(...)` call with the same name, URL and
`module_id`. Names like `"survie"`, `"https://s"` and module names appeared
dozens of times. This made tests noisy, and changing a fixture meant editing
every copy (with a real risk of missing one).

## Decision

Shared test data and setup live in `src/testutil.rs`:

- a `fixtures` module holding the reused literals (server name/URL, module
  names, description);
- `seed_principal`, `seed_server` and `server_request` helpers for the common
  module/server setup;
- HTTP test helpers (`server_body`, `create_server`) in `src/tests/mod.rs`.

Tests reference these instead of repeating literals or blocks. Values that are
deliberately distinct per test (e.g. two different URLs used to prove partial
index reuse, or distinct crypto inputs) stay local.

## Consequences

- A fixture change is a one-line edit.
- Tests read as intent (`seed_server(&pool, &owner)`) rather than setup noise.
- `testutil.rs` is `#[cfg(test)]` only and never ships to production.
- The threshold to extract a helper is "used in two or more places"; one-off
  values are kept inline to avoid over-abstracting.
