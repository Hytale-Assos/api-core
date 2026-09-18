# ADR-0011: REST over gRPC for ingestion

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

Event ingestion (player sessions) is done through an HTTP webhook. We considered
replacing it with gRPC (streaming, typed protobuf contracts, per-message acks)
for real-time presence and stronger typing.

## Decision

The transport stays **REST** (HTTP webhooks plus read endpoints). Reasons:

- Load is not a problem: Axum/Tokio absorbs the expected concurrency; the real
  bottleneck would be the SQLx pool, not the transport.
- A signed HTTP POST is trivial to emit from a third-party plugin; embedding a
  gRPC/HTTP-2/TLS client is heavier.
- Debugging and operations are easier with `curl` and HTTP logs.

## Consequences

- No real-time presence semantics, no typed protobuf contract; the 202 is an
  implicit acknowledgment.
- If true real-time presence or a typed data plane is ever needed, this ADR must
  be superseded before introducing gRPC.
