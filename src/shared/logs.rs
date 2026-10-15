//! Log messages, defined in one place.
//!
//! Tracing call sites must reference these constants instead of inlining
//! literals, so log wording stays consistent and easy to search.

// Bootstrap
pub const API_CORE_STARTED: &str = "api-core started";
pub const SEED_DISABLED: &str = "admin seed disabled: no module created automatically";
pub const ADMIN_SEEDED: &str = "admin module seeded";
pub const ADMIN_ALREADY_PRESENT: &str = "admin module already present";
pub const ADMIN_SEED_FAILED: &str = "admin seed failed";

// Modules
pub const MODULE_REGISTERED: &str = "module registered";

// Server agent registration / command queue
pub const SERVER_AGENT_REGISTERED: &str = "server agent registered";
pub const SERVER_AGENT_CLEARED: &str = "server agent cleared";
pub const QUEUE_BATCH_FAILED: &str = "whitelist queue batch delivery failed";
pub const QUEUE_DELIVERED: &str = "whitelist command delivered";
pub const QUEUE_RESCHEDULED: &str = "whitelist command rescheduled";
pub const QUEUE_FAILED: &str = "whitelist command failed permanently";
pub const QUEUE_MARK_FAILED: &str = "whitelist command state update failed";

// Whitelist
pub const WHITELIST_ENTRY_ADDED: &str = "whitelist entry added";
pub const WHITELIST_ENTRY_DELETED: &str = "whitelist entry deleted";

// Sessions
pub const SESSION_STARTED: &str = "session started";
pub const SESSION_ENDED: &str = "session ended";
