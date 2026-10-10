# ADR-0018: UBI10-micro container runtime

- Status: Accepted
- Date: 2026-10-10
- Deciders: api-core maintainers

## Context

ADR-0017 chose containerized deployment but left the runtime base at
`debian:bookworm-slim`, shipped only to carry the binary plus `curl` for the
image healthcheck (103 MB image). The production host is Rocky Linux 10.2, a
RHEL 10 rebuild. A benchmark (2026-10-10) ran the real release binary in
three candidate runtimes — `debian:bookworm-slim`, `gcr.io/distroless/cc-debian12`,
`ubi10/ubi-micro` — each against the dev Postgres with migrations applied:

- all three booted, applied migrations, served `/health` 200, ran non-root;
- distroless 35.3 MB, ubi10-micro 37 MB: functionally equivalent;
- the binary is dynamically linked against glibc 2.36 (bookworm build stage),
  so the runtime glibc must be >= the build glibc — this rules out UBI9
  (glibc 2.34) but makes UBI10 (glibc 2.39) a drop-in runtime;
- neither minimal runtime ships a shell, package manager, or curl.

## Decision

1. The production runtime is `registry.access.redhat.com/ubi10/ubi-micro`,
   pinned by digest; UBI rather than a Rocky image because only UBI offers a
   micro variant, and it carries Red Hat security errata with a long ABI
   guarantee matching the Rocky 10 host.
2. The image runs as a fixed non-root UID (`10001:0`); ubi-micro has no
   `useradd`, and the Quadlet hardening already assumes a non-root image.
3. The healthcheck no longer uses `curl`: the binary ships its own probe
   (`api-core healthcheck`, exit 0 on HTTP 200 of `/health`), wired through
   `HealthCmd` in the Quadlet unit. Image-level `HEALTHCHECK` metadata is
   dropped on OCI-format bases and is not used.
4. CA certificates are copied from a `ubi10-minimal` stage. Nothing reads
   them today (the queue worker delivers over plain HTTP and sqlx embeds
   webpki roots); they exist so future outbound HTTPS needs no image rebuild.
5. The build stage stays on `rust:1.94-slim-bookworm`. Any runtime swap must
   keep runtime glibc >= build glibc.

## Consequences

- Image shrinks from 103 MB to 37 MB; no shell, no package manager, no curl
  reduces the attack surface, and the userspace matches the deployment host.
- Debugging inside a running container is impossible (`podman exec` has no
  shell); diagnostics go through logs, host-side `podman top`/`podman mount`.
- The healthcheck depends on the binary's probe subcommand; it must not
  regress (covered by the CLI black-box tests in `tests/`).
- Bumping the base image means bumping tag and digest together, for both the
  runtime and the CA stage.
