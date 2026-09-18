# Architecture Decision Records (ADR)

This directory records the significant, durable decisions made on `api-core`.
An ADR is written **once**, then rarely edited: its value is the historical
record of *why* a choice was made. If a decision changes, add a new ADR that
supersedes the old one instead of rewriting it.

## When to write an ADR

Write one when a decision:

- shapes the architecture or module boundaries;
- constrains future work and is expensive to reverse;
- affects security, data integrity, or the public API contract;
- would otherwise be re-litigated in every review.

Do **not** write an ADR for routine refactors, naming, or local choices.

## Format

Each ADR follows a lightweight variant of the
[Michael Nygard format](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions):

```
# ADR-XXXX: Title

- Status: Proposed | Accepted | Superseded by ADR-YYYY
- Date: YYYY-MM-DD
- Deciders: <who>

## Context
## Decision
## Consequences
```

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](0001-hub-architecture.md) | Hub architecture with a modular monolith | Accepted |
| [0002](0002-single-authentication-model.md) | Single authentication model: everything is a module | Accepted |
| [0003](0003-no-user-accounts-in-core.md) | No user accounts in api-core | Accepted |
| [0004](0004-hytale-id-is-a-filter.md) | `hytale_id` is a data filter, not an identity | Accepted |
| [0005](0005-read-global-write-owned.md) | Reads are global, writes are ownership-restricted | Accepted |
| [0006](0006-service-layer-and-no-repository-trait.md) | Service layer, no repository trait | Accepted |
| [0007](0007-soft-delete-everywhere.md) | Soft delete everywhere | Accepted |
| [0008](0008-uuid-v7-primary-keys.md) | UUID v7 primary keys | Accepted |
| [0009](0009-application-level-encryption.md) | Application-level encryption and keyed hashing | Accepted |
| [0010](0010-outbox-for-outbound-effects.md) | Outbox pattern for outbound side effects | Accepted |
| [0011](0011-rest-over-grpc.md) | REST over gRPC for ingestion | Accepted |
| [0012](0012-configurable-admin-seed.md) | Configurable, idempotent admin seed | Accepted |
