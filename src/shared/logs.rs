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

// Whitelist
pub const WHITELIST_ENTRY_ADDED: &str = "whitelist entry added";
pub const WHITELIST_ENTRY_DELETED: &str = "whitelist entry deleted";

// Sessions
pub const SESSION_STARTED: &str = "session started";
pub const SESSION_ENDED: &str = "session ended";
