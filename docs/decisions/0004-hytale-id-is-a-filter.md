# ADR-0004: `hytale_id` is a data filter, not an identity

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

Clients (website, Discord bot) call the core with a `hytale_id` to ask for "this
player's sessions" or "this player's whitelist". Because the core does not
authenticate end users (ADR-0003), it cannot verify that the caller is acting on
behalf of that player.

It is tempting to treat the incoming `hytale_id` as an authenticated identity
and let it drive authorization.

## Decision

A `hytale_id` in a request is **only a data filter**. Authorization is always
derived from the authenticated `Principal` (the module) and its scopes. The core
never grants or denies access based on the value of a `hytale_id`.

Only trusted backends know and send `hytale_id` values. The end user never
transmits one directly, so there is no user-facing impersonation vector. This
trust boundary is part of the design and is the consumer's responsibility.

## Consequences

- Services never branch on "does this `hytale_id` belong to the caller".
- The trust model is explicit: any module with `player_sessions:read` can read
  any player's sessions. This is intentional and must be reflected when
  granting scopes.
- If the trust assumption ever changes (untrusted clients, per-player tokens),
  this ADR must be superseded and authorization reworked.
