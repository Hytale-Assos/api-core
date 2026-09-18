# ADR-0002: Single authentication model — everything is a module

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

The original implementation had two authentication systems:

- modules authenticated with an API key + HMAC signature;
- `/api/v1/admin/*` routes protected by a static `X-Admin-Token`.

This meant two auth paths, two authorization models, two sets of tests, and a
special case for the backoffice. Every new consumer (website, bot, backoffice)
widened the gap.

## Decision

There is exactly **one authentication model**: every caller is a **module**
identified by an API key, and every write is signed with HMAC-SHA256
(`"{timestamp}.{body}"`). Authorization is granted through **scopes**.

The static admin token and all `/api/v1/admin/*` routes are removed. The
backoffice is a regular module named `admin` that holds privileged scopes,
including the `admin` scope which bypasses ownership checks.

Reads (`GET`) require only the API key; writes (`POST`/`DELETE`) require API key
plus a valid HMAC signature within the replay window.

## Consequences

- One code path for authentication and authorization; one mental model.
- New consumers are onboarded by creating a module and granting scopes.
- The bootstrap problem (creating the first module) is solved by a
  configuration-driven seed (see ADR-0012).
- The `admin` scope becomes a small privilege-escalation surface and must be
  granted deliberately.
