# api-core — Hytale ecosystem hub

REST API centralizing Hytale servers, player sessions, and whitelists for all
consumers of the Hytale Assos ecosystem: the in-game server plugin, the member
website, the Discord bot, and the orchestrator. The core is the hub —
consumers call the core, the core calls no consumer.

Rust/Axum + PostgreSQL modular monolith. Every consumer is a **module**
authenticated by API key (reads) and HMAC signature (writes).

## Stack

- **Rust 2024** / **Axum 0.8** — HTTP server
- **SQLx 0.9** + **PostgreSQL 17** — data access, migrations applied at startup
- **AES-256-GCM** encryption + keyed HMAC hashing at the application layer
- **Docker Compose** — development database
- **Podman / Quadlet** — production deployment (`container/`)

## Prerequisites

- Rust 1.94+ (stable)
- Docker (for the dev database)

## Quick start

```bash
# 1. Copy the environment template and fill in ENCRYPTION_KEY
cp .env.example .env

# 2. Start the dev database (PostgreSQL 17 on localhost:5433)
docker compose up -d

# 3. Run the API (migrations are applied at startup)
cargo run
```

Health check: `GET http://127.0.0.1:3000/health` (public).

## Useful commands

| Command | Purpose |
| --- | --- |
| `cargo fmt` | Format the Rust code |
| `cargo clippy --all-targets` | Lint the Rust code |
| `cargo test` | Run the test suite (needs `DATABASE_URL`) |
| `bash scripts/install-hooks.sh` | Install the pre-commit formatting hook (once per clone) |
| `bash scripts/reset_dev.sh` | Wipe dev data and re-seed the plugin/website modules |

## Configuration

All configuration is environment-based (template: `.env.example`). Key values:

| Variable | Description |
| --- | --- |
| `BIND_ADDR` | HTTP listen address (default `0.0.0.0:3000`). |
| `DATABASE_URL` | PostgreSQL connection string. |
| `ENCRYPTION_KEY` | Master key for AES-256-GCM (base64, 32 bytes). Required; losing it makes the data unreadable. |
| `SEED_ADMIN_*` | Idempotent seed of the backoffice module (all scopes, incl. `admin`). Leave empty to seed nothing (ADR-0012). |
| `HMAC_TOLERANCE_SECS` | Anti-replay window for write signatures (default 300). |
| `CORS_ALLOWED_ORIGINS` | Comma-separated allowed origins (`*` to allow all). |
| `QUEUE_*` | Worker settings for management commands pushed to registered servers (ADR-0016). |

## Internal documentation

- [`docs/api.md`](docs/api.md) — endpoint reference and public contract
- [`docs/scopes.md`](docs/scopes.md) — scope catalog and authorization model
- [`docs/schema.md`](docs/schema.md) — database schema
- [`docs/architecture.md`](docs/architecture.md) — architecture overview
- [`docs/specs.md`](docs/specs.md) — functional specifications
- [`docs/decisions/`](docs/decisions/README.md) — Architecture Decision Records

## Conventions

- [`AGENTS.md`](AGENTS.md) — working rules (ADRs first, architecture, workflow)
- [`.ai/rules/`](.ai/rules/index.md) — path-scoped rules: commits, domains, migrations, tests, containers
- [`container/`](container/) — production image (Containerfile) and Quadlet units
