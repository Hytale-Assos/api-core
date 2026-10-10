//! CLI-level tests for the `healthcheck` mode of the `api-core` binary.
//!
//! These tests spawn the real binary as a child process — the only black-box
//! way to pin the `healthcheck` argument and the `BIND_ADDR` target
//! derivation — and observe its exit code. The child runs with a cleared
//! environment, so the healthcheck mode cannot silently depend on any
//! environment variable other than `BIND_ADDR`.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, ExitStatus};
use std::time::Duration;

/// Path of the binary under test, provided by Cargo for integration tests.
const BIN: &str = env!("CARGO_BIN_EXE_api-core");

/// Serves up to 8 requests with the given raw status line and no body, then
/// stops, so no listener leaks once the tests are done.
fn spawn_stub_server(listener: TcpListener, status_line: &'static str) {
    std::thread::spawn(move || {
        for _ in 0..8 {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            // Drain the incoming request (best effort) before answering.
            let mut request = [0u8; 2048];
            let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
            let _ = stream.read(&mut request);
            let response =
                format!("{status_line}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
}

/// Binds an ephemeral loopback port and returns it without holding a
/// listener on it.
fn reserve_ephemeral_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("local address").port()
}

/// Runs the binary with exactly the `healthcheck` argument and a cleared
/// environment; `bind_addr` is passed as `BIND_ADDR` when present.
fn run_healthcheck_cli(bind_addr: Option<&str>) -> ExitStatus {
    let mut command = Command::new(BIN);
    command.arg("healthcheck");
    command.env_clear();
    if let Some(addr) = bind_addr {
        command.env("BIND_ADDR", addr);
    }
    command.status().expect("spawn api-core healthcheck")
}

#[test]
fn healthcheck_exits_zero_when_probe_target_answers_200() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub server");
    let port = listener.local_addr().expect("local address").port();
    spawn_stub_server(listener, "HTTP/1.1 200 OK");

    // The environment is cleared: no DATABASE_URL, no encryption key, no
    // other variable. Reaching 0 here proves the mode neither requires the
    // usual server configuration nor touches the database.
    let status = run_healthcheck_cli(Some(&format!("127.0.0.1:{port}")));
    assert!(status.success(), "exit code should be 0, got {status}");
}

#[test]
fn healthcheck_exits_non_zero_when_nothing_listens() {
    let port = reserve_ephemeral_port();
    let status = run_healthcheck_cli(Some(&format!("127.0.0.1:{port}")));
    assert!(
        !status.success(),
        "connection refused must give a non-zero exit code, got {status}"
    );
}

#[test]
fn healthcheck_exits_non_zero_on_non_200_answer() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub server");
    let port = listener.local_addr().expect("local address").port();
    spawn_stub_server(listener, "HTTP/1.1 503 Service Unavailable");

    let status = run_healthcheck_cli(Some(&format!("127.0.0.1:{port}")));
    assert!(
        !status.success(),
        "HTTP 503 must give a non-zero exit code, got {status}"
    );
}

#[test]
fn healthcheck_probes_loopback_when_bound_to_zero_host() {
    // A host of "0.0.0.0" must be rewritten to 127.0.0.1: the stub listens
    // on the loopback interface only.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub server");
    let port = listener.local_addr().expect("local address").port();
    spawn_stub_server(listener, "HTTP/1.1 200 OK");

    let status = run_healthcheck_cli(Some(&format!("0.0.0.0:{port}")));
    assert!(
        status.success(),
        "0.0.0.0 must be probed as 127.0.0.1, got exit code {status}"
    );
}

#[test]
fn healthcheck_defaults_to_loopback_port_3000() {
    // Unset and empty BIND_ADDR both mean "0.0.0.0:3000", probed on the
    // loopback interface. Binding the port ourselves keeps the test
    // deterministic.
    let listener = TcpListener::bind("127.0.0.1:3000")
        .expect("port 3000 must be free to test the default target");
    spawn_stub_server(listener, "HTTP/1.1 200 OK");

    let unset = run_healthcheck_cli(None);
    assert!(
        unset.success(),
        "unset BIND_ADDR must probe 127.0.0.1:3000, got {unset}"
    );

    let empty = run_healthcheck_cli(Some(""));
    assert!(
        empty.success(),
        "empty BIND_ADDR must probe 127.0.0.1:3000, got {empty}"
    );
}
