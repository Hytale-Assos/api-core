# ADR-0010: Outbox pattern for outbound side effects

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

The planned orchestration domain will perform long, failure-prone outbound work:
fetching the latest Hytale server version and updating servers. Doing this
synchronously inside an HTTP request would block the caller and time out when
the external source is slow or unavailable.

## Decision

Any outbound side effect (network call, update job) must go through an
**outbox / jobs** table processed by a background worker. HTTP handlers only
enqueue work and return immediately; they never perform external calls in the
request path. Enqueued work must be **idempotent**, so a retried or duplicated
job cannot corrupt state (the same pattern already used for sessions and
whitelists).

## Consequences

- HTTP latency and reliability do not depend on external services.
- Failures are retried by the worker instead of failing the client request.
- Jobs need a status lifecycle, retry policy, and idempotency keys.
- The orchestration domain will require a migration for the outbox/jobs table
  and a worker process; both are deferred until that domain is implemented.
