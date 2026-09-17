-- Schéma initial de l'API core.
--
-- Principes :
-- - Clés primaires UUID v7 générées côté application (ordonnées dans le temps).
-- - Horodatage en TIMESTAMPTZ.
-- - Suppression logique via `deleted_at` ; désactivation via `is_enabled`.
-- - `updated_at` maintenu automatiquement par le trigger `set_updated_at()`.
-- - Chiffrement applicatif : seuls les secrets (voir `src/crypto.rs`) sont
--   chiffrés AES-256-GCM avant insertion. Le reste est stocké en clair.

-- Fonction utilitaire : met à jour `updated_at` à chaque UPDATE.
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Modules autorisés à parler au core (website, plugin Hytale, bot Discord...).
--
-- `api_key_hash` est un HMAC-SHA256 déterministe (index de recherche sûr) :
-- il permet de retrouver un module par API key sans stocker la clé en clair.
-- `hmac_secret` est chiffré AES-256-GCM, au format base64(nonce || ciphertext || tag).
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

-- Scopes accordés à un module (remplace l'ancien champ `kind`).
CREATE TABLE IF NOT EXISTS module_scopes (
    module_id  UUID NOT NULL REFERENCES modules (id) ON DELETE CASCADE,
    scope      TEXT NOT NULL,
    PRIMARY KEY (module_id, scope)
);

-- Serveurs Hytale gérés par un module.
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

-- Sessions de connexion des joueurs.
--
-- Le plugin n'émet une requête qu'une fois le joueur entré dans le monde :
-- `joined_at` est donc NOT NULL. `ended_at IS NULL` = session en cours.
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

-- Whitelist des joueurs, par serveur Hytale.
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
