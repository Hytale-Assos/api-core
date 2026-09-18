# ADR-0005: Reads are global, writes are ownership-restricted

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

Several modules manage Hytale servers. A server belongs to the module that
created it (`hytale_servers.module_id`). Consumers such as the website and the
Discord bot need to *display* every available server so a user can pick one, but
they must not be able to modify servers they do not own.

We had to decide the scope of read versus write operations.

## Decision

- **Reads are global.** Any module holding the relevant `:read` scope can list
  and inspect all servers, sessions, and whitelists. Reading is not risky.
- **Writes are ownership-restricted.** A module may only modify resources that
  belong to it (`assert_owned`). Creating a server is limited to the caller's
  own `module_id`.
- The `admin` scope bypasses ownership checks and is reserved for the
  backoffice.

An optional `module_id` query parameter lets a reader narrow a list on purpose.

## Consequences

- The website and bot can present all servers without holding `admin`.
- Granting a `:read` scope means granting visibility over all data of that
  kind; this must be a conscious choice.
- Ownership logic lives in one place (`servers::service::assert_owned`) and is
  reused by sessions and whitelists.
