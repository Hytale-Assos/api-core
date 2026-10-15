# ADR-0012: Configurable, idempotent admin seed

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

With a single authentication model (ADR-0002), creating a module requires being
an authenticated module. This is a bootstrap problem: something must create the
very first module, otherwise the API cannot be administered at all.

## Decision

At startup, the core seeds an `admin` module from environment variables:

- `SEED_ADMIN_API_KEY` and `SEED_ADMIN_HMAC_SECRET` (required together);
- `SEED_ADMIN_NAME` (optional, defaults to `admin`).

The seed is **idempotent**: if a module with that name already exists, nothing
happens. If the variables are absent, no module is created and a warning is
logged. The admin module receives all scopes, including `admin`.

Once bootstrapped, all other modules are managed through the API (`curl` or the
future backoffice module).

## Consequences

- A fresh deployment can be brought up with just environment variables.
- The seed secrets are sensitive configuration and must be rotated/stored like
  any credential; leaving them unset disables automatic bootstrap.
- No admin creation endpoint is needed, and there is no unauthenticated
  module-creation route.
