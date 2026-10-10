---
paths:
  - 'migrations/**'
---

# Database migrations

Migrations are raw SQL files in `migrations/`, applied at startup by
`sqlx::migrate!("./migrations")` — they are embedded in the binary at compile
time. Keep them forward-only: never edit an already-applied migration, add a
new one instead.

## Data rules (ADR-0007, ADR-0008, ADR-0009)

- **Soft delete only.** Every business table gets a nullable `deleted_at
  TIMESTAMPTZ`. Reads filter `deleted_at IS NULL` (see `.ai/rules/domains.md`).
- **Reusable names need a partial unique index**, so a soft-deleted name can
  be reused:
  `CREATE UNIQUE INDEX ... ON <table> (lower(name)) WHERE deleted_at IS NULL;`
- **Primary keys are UUID v7**, generated in the application, not by the
  database: `id UUID PRIMARY KEY` without a `DEFAULT`.
- **Never add a clear-text secret column.** Secrets are encrypted with
  AES-256-GCM in the application layer; searchable values get an HMAC
  `*_hash` column (blind index) instead.
- Timestamps are `TIMESTAMPTZ`, set by the application (`created_at`,
  `updated_at`).
