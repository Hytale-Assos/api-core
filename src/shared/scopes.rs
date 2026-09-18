//! Scopes recognized by the API. Storage stays free-form `TEXT` (extensible);
//! these constants are the code reference and feed the admin seed.

// Player sessions
pub const PLAYER_SESSIONS_READ: &str = "player_sessions:read";
pub const PLAYER_SESSIONS_WRITE: &str = "player_sessions:write";

// Whitelist
pub const WHITELIST_READ: &str = "whitelist:read";
pub const WHITELIST_WRITE: &str = "whitelist:write";

// Hytale servers
pub const HYTALE_SERVERS_READ: &str = "hytale_servers:read";
pub const HYTALE_SERVERS_WRITE: &str = "hytale_servers:write";

// Modules
pub const MODULES_READ: &str = "modules:read";
pub const MODULES_WRITE: &str = "modules:write";

/// Bypasses ownership checks (backoffice).
pub const ADMIN: &str = "admin";

/// All scopes, granted to the `admin` module from the seed.
pub const ALL: &[&str] = &[
    PLAYER_SESSIONS_READ,
    PLAYER_SESSIONS_WRITE,
    WHITELIST_READ,
    WHITELIST_WRITE,
    HYTALE_SERVERS_READ,
    HYTALE_SERVERS_WRITE,
    MODULES_READ,
    MODULES_WRITE,
    ADMIN,
];
