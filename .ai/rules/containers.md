---
paths:
  - 'container/**'
  - '.dockerignore'
  - '.github/workflows/build.yml'
---

# Containers

## Layout

- `container/Containerfile`: production image, OCI-generic (Podman first).
- `container/systemd/`: Quadlet files, copied at deploy time to
  `~/.config/containers/systemd/` (rootless) or `/etc/containers/systemd/`.
- `.dockerignore`: build context filter, must live at the context root, not
  next to the Containerfile.

## Containerfile

- Pin every base image as `tag@sha256:<digest>` using the multi-arch OCI
  index digest. Bump the tag and the digest together, in the same commit.
- Keep the version catalog as `ARG` lines at the top. `COPY --from=` does not
  support ARG expansion, but `FROM` does, so declare named source stages for
  each external image and copy from them by stage name.
- Two stages only: `build` (Rust toolchain) and the runtime. Migrations are
  embedded at compile time by `sqlx::migrate!`, so the runtime stage carries
  a single static-ish binary and needs no source tree, no toolchain, and no
  `migrations/` directory.
- Keep the dependency layers cacheable in this order: copy
  `Cargo.toml`/`Cargo.lock` and build a stub `fn main() {}` first, then copy
  `src/` + `migrations/` and build for real. A dependency change then
  rebuilds only the second pass.
- The runtime is unprivileged: ubi10-micro (glibc-only userspace, no shell,
  no package manager, no curl — ADR-0018), fixed non-root `USER` UID,
  unprivileged port (3000). The binary ships its own probe: the healthcheck
  runs `api-core healthcheck` (exit 0 = healthy), never curl.
- The runtime glibc must be at least as new as the build stage's glibc
  (the binary is dynamically linked). Build on bookworm (2.36), run on
  EL10 (2.39) is fine; the reverse is not.
- Defaults live in `ENV`: `BIND_ADDR=0.0.0.0:3000`, `RUST_LOG=info`. Never
  bake secrets or a `.env` into the image; the app reads plain environment
  variables.

## .dockerignore

Must exclude at minimum: `.env*` (secrets), `target/` (huge and
machine-specific), `docs/`, `container/`, tests, dotfolders, `.githooks`,
`scripts`. `migrations/` and `src/` must stay: the build stage compiles
them.

## Verification

Before pushing a Containerfile change: build the image, then smoke-test it:
run the container with a `DATABASE_URL` and `ENCRYPTION_KEY`, check the
startup logs apply the migrations, hit `/health` for HTTP 200, and confirm
the process user from the host with `podman top <container> user` (the
runtime image has no `id` binary, `podman exec` is unavailable).

## Quadlet files

- One systemd unit per file; the unit name equals the file name. Prefix
  everything with `hytale-`. Container names: `hytale-api-core` (app) and
  `hytale-api-core-postgres` (database).
- Peer networks linking exactly two containers are named
  `hytale-<container-a>-hytale-<container-b>`:
  `hytale-api-core-hytale-api-core-postgres.network` (app ↔ database),
  `hytale-website-hytale-api-core.network` (website ↔ core),
  `hytale-server-vanilla-1-hytale-api-core.network` (game server ↔ core).
  Never put all services on one shared flat network: each dependency pair
  gets its own network, and consumers reach the core at
  `http://hytale-api-core:3000`. Each game server gets its own pair
  network (instance suffix `-1`, `-2`, ...): two game servers must never
  share a network, or they could reach each other's management agent.
- Volumes are named after their owning container:
  `hytale-api-core-postgres-data.volume`.
- Secrets never live in the repo. Sensitive values are Podman secrets
  injected with `Secret=<name>,type=env,target=<VAR>` and provisioned on
  the host with `podman secret create`. They are resolved at container
  creation: rotation means recreating the secret and restarting the unit. A
  missing secret makes systemd fail to start the unit.
- Secret names follow the unit prefix: `hytale-api-core-*`
  (`hytale-api-core-database-url`, `hytale-api-core-encryption-key`,
  `hytale-api-core-seed-admin-api-key`,
  `hytale-api-core-seed-admin-hmac-secret`). `DATABASE_URL` embeds the
  password, so the whole URL is a secret, not just the password.
- Non-secret configuration lives in an env file on the host, provisioned
  from `container/api-core.env.example`; the template never contains secret
  values.
- Rootless deployment layout: quadlets in `~/.config/containers/systemd/`,
  env files in `~/hytale/env/`, secret source files in `~/hytale/secrets/`.
- Wire dependencies in the `[Unit]` section (`Requires=`/`After=` on the
  generated service names). `Restart=always` goes in `[Service]`. Every
  container declares a `HealthCmd`.
- Hardening is mandatory: `NoNewPrivileges=true` everywhere; run application
  images as their non-root user so `DropCapability=ALL` is safe;
  `ReadOnly=true` with a `Tmpfs=` per writable path (`/tmp` here). Do not
  pass `uid=`/`gid=` to `Tmpfs=`; use `mode=1777` where a non-root process
  must write.
- `AutoUpdate=registry` only on the application container, never on the
  database.
- Escape environment variables in systemd-inherited values as `$$`
  (`HealthCmd=curl -fsS http://127.0.0.1:3000/health`).

## CI workflow

`.github/workflows/ci.yml` runs `cargo fmt --check`, `cargo clippy
--all-targets`, and `cargo test` (against a Postgres service) on every
pull request targeting `develop` and every push to `develop`. Toolchains run
inside digest-pinned container images; actions are pinned by commit SHA.

`.github/workflows/build.yml` builds the image on every pull request
targeting `develop` and on every push to `develop` (verification only, no
publish), and builds and pushes to `ghcr.io/hytale-assos/api-core` on every
push to `main` with two tags: `latest` and `sha-<short commit>`. Pin GitHub
Actions by commit SHA with the version in a comment. The registry namespace
is lowercase.
