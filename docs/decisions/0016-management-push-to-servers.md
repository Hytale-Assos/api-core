# ADR-0016: Pushing management commands to registered servers

- Status: Accepted
- Date: 2026-09-20
- Deciders: core maintainers

## Context

The whitelist is stored in `api-core` (source of truth) but the native Hytale
whitelist lives inside each running server, exposed by the server management
plugin over its loopback REST API. When a member adds themselves to a whitelist
from the website, the core must make the corresponding server actually grant
join access.

ADR-0001 states the core is a hub and "calls no consumer". ADR-0010 forbids
outbound network calls inside an HTTP request path and mandates an outbox +
worker for outbound side effects. Those two decisions together left no path for
the core to reach a server.

## Decision

The core may push **management commands** to servers whose **agent** (the
server's control plane) has explicitly been linked. This is a narrow, opt-in
exception to ADR-0001:

- The agent of a server is exposed as a sub-resource:
  `PUT|GET|DELETE /api/v1/servers/{id}/agent`. `PUT` registers (or replaces) the
  agent control-plane URL and bearer token; `GET` returns the status without the
  token; `DELETE` unlinks. The token is stored **AES-256-GCM encrypted**
  (ADR-0009). Ownership rules apply: only the module owning the server (or
  `admin`) may link it.
- Linking is **operator-driven**, not automatic. The core never discovers
  consumers on its own; a consumer that starts without being linked has no
  effect on the core. Automation (when it exists) belongs to the orchestrator,
  which knows the URL and has the intent.
- Every push goes through a **`whitelist_queue`** table processed by a
  background worker (ADR-0010). HTTP handlers only enqueue; they never perform
  the outbound call. Delivery is at-least-once and idempotent on the agent side
  (adding an already-whitelisted player is a no-op).
- A server that is not linked is left pending without consuming a delivery
  attempt, so a queued command survives until the agent links.
- `reqwest` is deliberately not added; the worker uses `hyper` (already present
  transitively) with a small client wrapper.

This supersedes the "calls no consumer" clause of ADR-0001, and refines
ADR-0005 for whitelists.

### Whitelist is self-service

Whitelist entries are **not** ownership-restricted: a module holding
`whitelist:write` may add or remove an entry for any existing server, because
whitelisting is a member self-service action (a member whitelists themselves on
a server they do not own). Server creation and deletion remain
ownership-restricted per ADR-0005.

The entry's `hytale_server_id` must still reference an existing, non-deleted
server; this is a referential integrity check, not an ownership check.

## Consequences

- The website can add a member to any server's whitelist without holding
  `admin`.
- The core gains a minimal outbound capability, isolated in the worker; HTTP
  request latency never depends on a server being online.
- A new migration adds agent columns and the `whitelist_queue` table, plus a
  worker spawned at startup.
- Servers that are never linked cannot be reached; the worker leaves their
  commands pending instead of failing.
- Future orchestrations (updates, restarts) should reuse the same agent link +
  queue mechanism rather than inventing another channel; the orchestrator is
  the component that links agents at provisioning time.
