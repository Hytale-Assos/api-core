# ADR-0017: Containerized deployment with Podman/Quadlet

- Status: Accepted
- Date: 2026-10-10
- Deciders: core maintainers

## Context

The website project (`hytale/website`) already deploys as a hardened,
rootless Podman/Quadlet setup: digest-pinned images, non-root runtimes,
read-only root filesystems, Podman secrets, and pair networks. The two
projects share a host, an operations model, and the same consumers; deploying
`api-core` with a different mechanism would double the operational surface.

`api-core` needs a production story beyond the dev `docker-compose.yml`
(which only provides PostgreSQL): a single self-contained binary, no
`.env`-based provisioning on the host, and migrations applied at startup
(`sqlx::migrate!` embeds them at compile time, so the image needs no source
tree and no migration step).

## Decision

Deploy `api-core` with the same Podman/Quadlet model as the website:

- **Image** (`container/Containerfile`): two digest-pinned stages — a Rust
  build stage and a `debian:bookworm-slim` runtime carrying only the binary
  and `curl` (healthcheck). The runtime runs as a dedicated non-root user on
  port 3000. Dependency layers are cached behind a stub build. No secrets and
  no `.env` are ever baked in: configuration comes from environment variables.
- **Quadlet units** (`container/systemd/`), all `hytale-` prefixed:
  - `hytale-api-core.container` — the app, `ReadOnly=true` rootfs, `/tmp`
    tmpfs, `DropCapability=ALL`, `AutoUpdate=registry`.
  - `hytale-api-core-postgres.container` + its data volume — the database.
  - `hytale-api-core-hytale-api-core-postgres.network` — the only network
    between the app and its database.
  - `hytale-website-hytale-api-core.network` — the peer network over which
    the website reaches the core at `http://hytale-api-core:3000`. The
    website's own quadlet must join it; that edit belongs to the website
    repository.
- **Secrets**: `DATABASE_URL` (it embeds the password), `ENCRYPTION_KEY`, and
  the two `SEED_ADMIN_*` values are Podman secrets injected as env vars, per
  ADR-0009 and ADR-0012. Everything else ships in a host env file
  (`container/api-core.env.example` as template). Empty seed secrets disable
  the admin seed.
- **CI/CD**: `ci.yml` runs fmt/clippy/test on every PR and push to
  `develop`, inside the same digest-pinned toolchain image. `build.yml` builds
  the image on PRs (verification only) and pushes
  `ghcr.io/hytale-assos/api-core` with `latest` and `sha-<short>` tags on
  every push to `main`. Actions are pinned by commit SHA.

## Consequences

- The two Hytale projects share one deployment and operations model; skills
  and hardening checks transfer between them.
- Upgrading the core in production is pulling a new image and letting
  `podman-auto-update` roll it forward; migrations run at startup, so the
  database must be reachable before the app unit starts (`Requires=`/
  `After=` wiring).
- Base images (Rust, Debian, PostgreSQL) are digest-pinned: bumps are
  deliberate, tag and digest move together in one commit.
- The host port 3000 stays published because the in-game plugin and other
  external consumers are not on the Quadlet networks.
- Quadlet files are deployment templates, not active host state: they are
  copied to `~/.config/containers/systemd/` at deploy time, and secrets/env
  files are provisioned on the host, never committed.
