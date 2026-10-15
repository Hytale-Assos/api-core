# ADR-0007: Soft delete everywhere

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

Rows such as servers, sessions, and whitelist entries must be removable while
keeping history and referential integrity. Hard deletes would break foreign keys
and erase audit-relevant data.

## Decision

Deletion is **logical only**: rows carry a nullable `deleted_at TIMESTAMPTZ` and
are never physically removed from the tables. Every read filters
`deleted_at IS NULL`. Uniqueness constraints that must ignore deleted rows are
implemented as partial unique indexes, e.g.
`UNIQUE (module_id, name) WHERE deleted_at IS NULL`, so a name can be reused
after deletion.

`is_enabled` is a separate, reversible state (e.g. disable a module) and is not
the same as deletion.

## Consequences

- History and foreign keys are preserved.
- Every query must remember the `deleted_at IS NULL` filter; this is enforced by
  convention in `repo.rs` and covered by tests.
- Tables grow without bound. Retention/purge (GDPR) is an open point tracked in
  `docs/schema.md`.
