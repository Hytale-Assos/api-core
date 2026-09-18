# ADR-0014: English as the single project language

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

Code comments, user-facing messages, SQL migration comments, documentation and
environment examples were partly in French. The core is a hub consumed by
several services (Hytale plugin, Laravel site, Discord bot, future
orchestration) and may be read or contributed to by people outside the original
author's language. Mixed languages hurt searchability and consistency, and a
French comment plus an English identifier for the same concept invites drift.

## Decision

Everything committed to the repository is written in **English**:

- Rust code: identifiers and comments;
- user-facing error messages and log messages;
- SQL and migration comments;
- documentation under `docs/` and `AGENTS.md`;
- configuration examples (`.env.example`) and commit/PR conventions.

Commit messages are English and imperative (see
[`docs/git-conventions.md`](../git-conventions.md)).

The only tolerated non-English content is the local `.env` file, which is never
committed.

## Consequences

- The repository is consistently readable by an international audience.
- A language check is worth adding to CI later (grep for non-ASCII outside
  typography); for now it is a review responsibility.
- Historical documents already written in another language would need
  translation before being reintroduced; the docs were translated in the same
  change as this ADR.
