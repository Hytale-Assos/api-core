# ADR-0013: Centralized user-facing messages and logs

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

Error messages were inlined at their call sites, for example
`AppError::BadRequest("request body could not be read".into())`. Log messages
were inlined too, for example `tracing::info!(..., "module registered")`. The
same idea often existed in several places with slightly different wording, and
changing a public message meant grepping the whole codebase. Tests also
repeated fixture literals (`"survie"`, `"https://s"`, module names).

## Decision

- **User-facing error messages** live in one place: the `messages` module in
  `src/error.rs`. Call sites reference the constants; they never inline a
  message. `AppError::public()` composes the HTTP status and message from those
  constants, so the public error contract is defined in a single location.
- **Log messages** live in `src/shared/logs.rs`. Tracing call sites reference
  the constants instead of literals.
- **Test fixtures** shared across tests live in `src/testutil.rs`
  (`fixtures` module). Tests reference them instead of repeating literals.

The rule is recorded in `AGENTS.md`: never inline a literal message at the call
site.

## Consequences

- The public error contract and log wording are easy to find and change.
- Renaming or translating a message is a one-line change.
- A new message requires adding a constant first; this is intentional friction
  that keeps the vocabulary consistent.
- Log messages are not internationalized (they stay technical, in English);
  the decision concerns keeping them centralized, not translating them.
