//! Queue of management commands pushed to registered servers (ADR-0016).
//!
//! Whitelist changes are enqueued transactionally next to the business write,
//! then delivered by the background worker. HTTP handlers never perform the
//! outbound call (ADR-0010).

pub mod model;
pub mod repo;
pub mod service;
pub mod worker;
