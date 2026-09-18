# Specs — Rust API (Axum)

Last verification: 15 September 2026

## 1. Goal

HTTP API in Rust able to:
- handle several requests simultaneously (multi-thread / async);
- expose a webhook endpoint (receiving POST events);
- connect to a relational database.

## 2. Chosen stack and maintenance status

| Crate | Role | Latest version (Sept. 2026) | Status |
|---|---|---|---|
| `axum` | Web framework | **0.8.7** | ✅ Actively maintained by the Tokio team, regular releases |
| `tokio` | Async / multi-thread runtime | **1.53.1** | ✅ Very actively maintained. Two LTS branches in progress: 1.47.x (supported until Sept. 2026) and 1.51.x (supported until March 2027) |
| `sqlx` | Async DB access | **0.9.0** (0.8.6 still widespread) | ✅ Maintained, but the release process is partly manual — always check the changelogs on a major upgrade |
| `tower` / `tower-http` | Middlewares (logs, CORS, timeouts...) | `tower` 0.5.3 / `tower-http` 0.7.1 | ✅ Maintained, natively integrated into the Axum ecosystem |
| `serde` / `serde_json` | JSON serialization | **1.0.229** | ✅ Extremely stable, maintained for years, de facto standard |
| `axum-extra` (optional) | Axum utilities (cookies, typed headers...) | 0.12.6 | ✅ Maintained alongside Axum |

**Overall verdict: the whole proposed stack is actively maintained in September
2026.** No sign of abandonment. Point of caution: `sqlx` sometimes publishes
major versions (0.8 → 0.9), so pin the version in `Cargo.toml` and read the
changelog before upgrading.

## 3. Framework choice: Axum vs Actix-web

| Criterion | Axum | Actix-web |
|---|---|---|
| Concurrency model | Single multi-thread Tokio runtime, work-stealing | Actors / several single-thread Tokio runtimes |
| Raw perf (saturated req/s) | ~18,000 | ~20,000 |
| Idle RAM | 60–90 MB | 50–80 MB |
| Ecosystem | Tower/Hyper/SQLx — consistent with "standard" async Rust | More self-contained, historical |
| Maintainability / learning curve | Simpler, handlers = regular async functions | More complex (actor model), but very performant |

**Decision: Axum.** The perf gap (~10-15%) has no real impact for the majority
of APIs (the bottleneck is almost always the database or the business logic,
not the framework). Axum offers better ergonomics and a more natural
integration with SQLx and Tower.

## 4. Multi-threading / concurrency

- No manual thread management required.
- The Tokio runtime (`#[tokio::main]`, default `multi_thread` flavor) distributes
  each incoming request as a lightweight async task over a pool of OS threads,
  with a *work-stealing* scheduler.
- Each `async fn` handler in Axum can run concurrently without blocking — as
  long as synchronous blocking code is avoided (otherwise use
  `tokio::task::spawn_blocking`).

```rust
#[tokio::main] // multi-thread by default
async fn main() {
    // ...
}
```

## 5. Database connection

- Chosen driver: **SQLx** (native async, compatible with Postgres / MySQL /
  SQLite, compile-time query verification).
- Alternative if a real typed ORM is needed: **SeaORM** (built on SQLx).
- Connection pool managed through `sqlx::Pool`, shared across handlers through
  the Axum state (`State`).

```rust
let pool = sqlx::postgres::PgPoolOptions::new()
    .max_connections(10)
    .connect(&database_url)
    .await?;
```

## 6. Webhook endpoint

- A webhook = a regular HTTP POST route exposed by the API.
- Best practices:
  - verify the request signature (HMAC header provided by the webhook sender);
  - respond quickly (200 OK) then process the event asynchronously if the
    processing is long, to avoid timeouts / retries on the sender side;
  - log the received payloads for debugging.

```rust
async fn webhook_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<WebhookPayload>,
) -> StatusCode {
    // 1. verify the signature (HMAC header)
    // 2. process / store the event
    StatusCode::OK
}
```

## 7. Suggested next steps

1. Generate the project skeleton (`Cargo.toml` + `main.rs` + folder structure).
2. Define the database schema and migrations (`sqlx-cli`).
3. Implement the main routes + the webhook endpoint.
4. Add Tower middlewares (logs, CORS, rate limiting) as needed.
5. Integration tests + deployment config.

## 8. Sources consulted

- Axum vs Actix-web comparisons 2026 (Medium, Rustify)
- crates.io / docs.rs for version numbers (axum, tokio, sqlx, serde, tower)
