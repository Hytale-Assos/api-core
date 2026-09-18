-- Initial schema of the API core.
--
-- Principles:
-- - UUID v7 primary keys generated in the application (time-ordered).
-- - TIMESTAMPTZ timestamps.
-- - Soft delete through `deleted_at`; disabling through `is_enabled`.
-- - `updated_at` maintained automatically by the `set_updated_at()` trigger.
-- - Application-level encryption: only secrets (see `src/crypto.rs`) are
--   AES-256-GCM encrypted before insertion. The rest is stored in the clear.

-- Utility function: updates `updated_at` on every UPDATE.
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Modules allowed to talk to the core (website, Hytale plugin, Discord bot...).
--
-- `api_key_hash` is a deterministic HMAC-SHA256 (safe lookup index): it allows
-- finding a module by API key without storing the key in the clear.
-- `hmac_secret` is AES-256-GCM encrypted, as base64(nonce || ciphertext || tag).
CREATE TABLE IF NOT EXISTS modules (
    id            UUID PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE,
    description   TEXT,
    api_key_hash  TEXT NOT NULL UNIQUE,
    hmac_secret   TEXT NOT NULL,
    is_enabled    BOOLEAN NOT NULL DEFAULT TRUE,
    deleted_at    TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER trg_modules_updated_at
    BEFORE UPDATE ON modules
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Scopes granted to a module (replaces the old `kind` field).
CREATE TABLE IF NOT EXISTS module_scopes (
    module_id  UUID NOT NULL REFERENCES modules (id) ON DELETE CASCADE,
    scope      TEXT NOT NULL,
    PRIMARY KEY (module_id, scope)
);

-- Hytale servers managed by a module.
CREATE TABLE IF NOT EXISTS hytale_servers (
    id          UUID PRIMARY KEY,
    module_id   UUID NOT NULL REFERENCES modules (id),
    name        TEXT NOT NULL,
    url         TEXT NOT NULL,
    deleted_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER trg_hytale_servers_updated_at
    BEFORE UPDATE ON hytale_servers
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE UNIQUE INDEX IF NOT EXISTS uq_hytale_servers_module_name
    ON hytale_servers (module_id, name)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_hytale_servers_module_id
    ON hytale_servers (module_id);

-- Player connection sessions.
--
-- The plugin only sends a request once the player has entered the world:
-- `joined_at` is therefore NOT NULL. `ended_at IS NULL` = session in progress.
CREATE TABLE IF NOT EXISTS player_sessions (
    id                UUID PRIMARY KEY,
    hytale_server_id  UUID NOT NULL REFERENCES hytale_servers (id),
    hytale_id         UUID NOT NULL,
    joined_at         TIMESTAMPTZ NOT NULL,
    ended_at          TIMESTAMPTZ,
    deleted_at        TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER trg_player_sessions_updated_at
    BEFORE UPDATE ON player_sessions
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX IF NOT EXISTS idx_player_sessions_server_joined
    ON player_sessions (hytale_server_id, joined_at DESC);

CREATE INDEX IF NOT EXISTS idx_player_sessions_hytale_id
    ON player_sessions (hytale_id);

CREATE INDEX IF NOT EXISTS idx_player_sessions_active
    ON player_sessions (ended_at)
    WHERE ended_at IS NULL;

-- Player whitelist, per Hytale server.
CREATE TABLE IF NOT EXISTS whitelists (
    id                UUID PRIMARY KEY,
    hytale_server_id  UUID NOT NULL REFERENCES hytale_servers (id),
    hytale_id         UUID NOT NULL,
    deleted_at        TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER trg_whitelists_updated_at
    BEFORE UPDATE ON whitelists
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE UNIQUE INDEX IF NOT EXISTS uq_whitelists_server_hytale
    ON whitelists (hytale_server_id, hytale_id)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_whitelists_hytale_id
    ON whitelists (hytale_id);
