//! Tests for the container health probe exposed at the crate root
//! (`run_healthcheck`): an HTTP `GET /health` against a `host:port` target,
//! returning `true` only when an HTTP 200 answer arrives within 5 seconds.

use std::time::{Duration, Instant};

use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use crate::http::router;
use crate::testutil::test_state;

/// Spawns the real application router (with its database pool) on an
/// ephemeral loopback port. Returns the `host:port` target and the server
/// task, which the caller aborts when done.
async fn spawn_app(pool: PgPool) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local address");
    let app = router::build(test_state(pool));
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve app");
    });
    (format!("127.0.0.1:{}", addr.port()), handle)
}

/// Spawns a stub server answering every `GET /health` with `status`.
async fn spawn_status_server(status: StatusCode) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local address");
    let app = Router::new().route("/health", get(move || async move { status }));
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve stub");
    });
    (format!("127.0.0.1:{}", addr.port()), handle)
}

/// Spawns a server that accepts connections but never answers, to force the
/// probe to hit its timeout.
async fn spawn_silent_server() -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local address");
    let handle = tokio::spawn(async move {
        // Hold every accepted socket open without ever writing a response.
        let mut held: Vec<tokio::net::TcpStream> = Vec::new();
        while let Ok((socket, _)) = listener.accept().await {
            held.push(socket);
        }
    });
    (format!("127.0.0.1:{}", addr.port()), handle)
}

#[sqlx::test]
async fn probe_succeeds_against_live_app(pool: PgPool) {
    let (target, server) = spawn_app(pool).await;
    assert!(
        crate::run_healthcheck(&target).await,
        "probe against a live app must succeed"
    );
    server.abort();
}

#[tokio::test]
async fn probe_accepts_any_200_response() {
    let (target, server) = spawn_status_server(StatusCode::OK).await;
    assert!(
        crate::run_healthcheck(&target).await,
        "any HTTP 200 answer must make the probe succeed"
    );
    server.abort();
}

#[tokio::test]
async fn probe_fails_on_non_200_status() {
    for status in [
        StatusCode::NOT_FOUND,
        StatusCode::INTERNAL_SERVER_ERROR,
        StatusCode::SERVICE_UNAVAILABLE,
    ] {
        let (target, server) = spawn_status_server(status).await;
        assert!(
            !crate::run_healthcheck(&target).await,
            "HTTP {} must make the probe fail",
            status
        );
        server.abort();
    }
}

#[tokio::test]
async fn probe_fails_on_connection_refused() {
    // Reserve an ephemeral port, then close it: nothing listens there anymore.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local address");
    drop(listener);

    let target = format!("127.0.0.1:{}", addr.port());
    assert!(
        !crate::run_healthcheck(&target).await,
        "a refused connection must make the probe fail"
    );
}

#[tokio::test]
async fn probe_fails_after_timeout_when_server_never_answers() {
    let (target, server) = spawn_silent_server().await;

    let started = Instant::now();
    let probed = tokio::time::timeout(Duration::from_secs(15), crate::run_healthcheck(&target))
        .await
        .expect("the probe must return instead of hanging");
    assert!(
        !probed,
        "a server that never answers must make the probe fail"
    );
    assert!(
        started.elapsed() >= Duration::from_secs(4),
        "the probe should have hit its 5s timeout, but returned after {:?}",
        started.elapsed()
    );

    server.abort();
}
