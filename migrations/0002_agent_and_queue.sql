-- Agent registration and management command queue (ADR-0016).
--
-- A server's control plane ("agent") registers its endpoint and bearer token
-- with the core. The token is AES-256-GCM encrypted at the application level
-- (ADR-0009). Whitelist changes are queued in `whitelist_queue` and delivered
-- by a background worker (ADR-0010): HTTP handlers only enqueue.

ALTER TABLE hytale_servers
    ADD COLUMN IF NOT EXISTS agent_url TEXT,
    ADD COLUMN IF NOT EXISTS agent_token TEXT,
    ADD COLUMN IF NOT EXISTS agent_registered_at TIMESTAMPTZ;

-- One pending command per (server, player, action): at-least-once delivery
-- without piling duplicates when a member toggles repeatedly.
CREATE TABLE IF NOT EXISTS whitelist_queue (
    id                UUID PRIMARY KEY,
    hytale_server_id  UUID NOT NULL REFERENCES hytale_servers (id),
    hytale_id         UUID NOT NULL,
    action            TEXT NOT NULL CHECK (action IN ('add', 'remove')),
    status            TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sent', 'failed')),
    attempts          INTEGER NOT NULL DEFAULT 0,
    last_error        TEXT,
    next_attempt_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER trg_whitelist_queue_updated_at
    BEFORE UPDATE ON whitelist_queue
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX IF NOT EXISTS idx_whitelist_queue_pending
    ON whitelist_queue (next_attempt_at)
    WHERE status = 'pending';

CREATE UNIQUE INDEX IF NOT EXISTS uq_whitelist_queue_open
    ON whitelist_queue (hytale_server_id, hytale_id, action)
    WHERE status = 'pending';
