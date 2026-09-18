# ADR-0003: No user accounts in api-core

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

The website (Laravel) manages user accounts in its own database. Users
authenticate there, including any "Sign in with Hytale" flow. The Discord bot
maps Discord users to players on its side. `api-core` is shared by all of them.

A naive design would introduce a `players` or `users` table in the core so that
"who is this player" is answerable centrally.

## Decision

`api-core` stores **no user accounts and no authentication of end users**. It
knows only modules (machine callers) and opaque `hytale_id` values. There is no
`players` table; `hytale_id` is denormalized into `player_sessions` and
`whitelists`.

Account creation, OAuth, credentials, and the mapping between a user account
(or Discord user) and a `hytale_id` all live in the consumer. The core is told a
`hytale_id` and acts on it.

## Consequences

- The core has no password/session/credential surface exposed to end users.
- No extra SQL round-trip (parse → lookup player → insert) on the hot path.
- If core-owned player attributes (display name, preferences, first/last seen)
  are ever needed, a `players` table will have to be reintroduced and
  `hytale_id` migrated to a `player_id`. This is a known, accepted trade-off.
- Consumers are trusted to send correct `hytale_id` values; see ADR-0004.
