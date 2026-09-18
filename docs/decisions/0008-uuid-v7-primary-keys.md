# ADR-0008: UUID v7 primary keys

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

Primary keys must be globally unique, non-guessable, and efficient as database
indexes. UUID v4 is random and causes index fragmentation on insert. Auto-
increment integers leak volume and require coordination across services.

## Decision

All tables use **UUID v7** primary keys, generated in the application with
`Uuid::now_v7()`. UUID v7 is time-ordered, so inserts have good index locality,
while remaining non-guessable and uniform for `sqlx::FromRow`.

## Consequences

- IDs sort by creation time and index well.
- IDs are generated in Rust, not by the database.
- Legacy v4 IDs remain valid (a UUID is a UUID).
- The `uuid` crate must keep the `v7` feature enabled.
