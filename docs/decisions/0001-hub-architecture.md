# ADR-0001: Hub architecture with a modular monolith

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

`api-core` is consumed by several independent clients: a Hytale plugin (machine
events), a Laravel website, a Discord bot, and (planned) an orchestration
service that updates Hytale servers. Future clients are expected. Clients must
not talk to each other directly; each one has different needs and its own
database and account system.

We had to choose between:

1. a modular monolith where the core hosts the logic of each consumer;
2. a hub + BFF model where each client is a separate service that *consumes*
   the core's API.

## Decision

`api-core` is the **hub**. It owns the business data (Hytale servers, player
sessions, whitelists, modules) and exposes it through a single HTTP API. The
plugin, the Laravel site, and the Discord bot are **external consumers** in
their own repositories; they call the core, and the core calls none of them.

The core is organized as a **modular monolith** by business domain
(`domains/<domain>`), not by client. Consumer-specific logic (OAuth, account
management, Discord commands, web pages) stays in the consumer.

## Consequences

- One source of truth for Hytale data; clients stay decoupled and replaceable.
- Adding a consumer does not require touching existing domains, only
  registering a module and granting scopes.
- The core stays small and focused: it must resist hosting client-specific
  features. Client logic belongs in the client.
- Cross-client features go through the core, never peer-to-peer.
