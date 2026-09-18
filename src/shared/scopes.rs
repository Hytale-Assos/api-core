//! Scopes reconnus par l'API. Le stockage reste du `TEXT` libre (extensible),
//! ces constantes servent de référence côté code et pour le seed admin.

// Sessions joueur
pub const PLAYER_SESSIONS_READ: &str = "player_sessions:read";
pub const PLAYER_SESSIONS_WRITE: &str = "player_sessions:write";

// Whitelist
pub const WHITELIST_READ: &str = "whitelist:read";
pub const WHITELIST_WRITE: &str = "whitelist:write";

// Serveurs Hytale
pub const HYTALE_SERVERS_READ: &str = "hytale_servers:read";
pub const HYTALE_SERVERS_WRITE: &str = "hytale_servers:write";

// Modules
pub const MODULES_READ: &str = "modules:read";
pub const MODULES_WRITE: &str = "modules:write";

/// Contourne les contrôles d'ownership (backoffice).
pub const ADMIN: &str = "admin";

/// Tous les scopes, accordés au module `admin` issu du seed.
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
