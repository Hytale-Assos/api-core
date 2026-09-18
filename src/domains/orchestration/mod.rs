//! Orchestration domain (coming soon).
//!
//! Goal: update Hytale servers (fetch the latest published version, schedule
//! and track updates).
//!
//! Design constraint: outbound network calls must **never** block an HTTP
//! request. They will go through an `outbox`/`jobs` table and a worker. The
//! seed and tests will land with the real implementation.
