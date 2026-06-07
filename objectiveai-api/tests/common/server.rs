//! Connects to the test api server spawned by `objectiveai-api/test.sh`
//! (or inherited from a parent harness, e.g. the root `test.sh`).
//!
//! All integration test binaries share **one** api server instance —
//! `test.sh` boots it via `test-spawn-api-server.sh` and exports
//! `OBJECTIVEAI_TEST_PORT`, which we read here to build the HTTP client.
//! Spawning the binary per-binary used to live here (one server per
//! `tests/*.rs` cargo binary) but contention on Windows ephemeral ports
//! during the agent_completions cluster surfaced as transient
//! ConnectionRefused panics — a single shared server avoids that
//! entirely. Cross-binary state isolation is handled inside the api
//! (per-request) and inside the tests (per-test directories), not at
//! the OS process level.

use std::sync::Arc;

use objectiveai_sdk::HttpClient;

/// Resolve the shared api server's base URL from the
/// `OBJECTIVEAI_TEST_PORT` env var. Panics with a remediation hint if
/// the variable isn't set — running these tests through raw
/// `cargo test` (no `test.sh` wrapper, no inherited harness) won't
/// work.
fn resolve_base_url() -> String {
    match std::env::var("OBJECTIVEAI_TEST_PORT") {
        Ok(port) if !port.is_empty() => format!("http://127.0.0.1:{port}"),
        _ => panic!(
            "OBJECTIVEAI_TEST_PORT is not set. Run via `bash objectiveai-api/test.sh` \
             (or `bash test.sh` at the repo root) so the shared api server is spawned \
             before the integration tests run."
        ),
    }
}

/// Returns a fresh `HttpClient` pointing at the shared test api server.
/// Each call builds its own reqwest pool — the calling test's runtime
/// is what hosts hyper's dispatch tasks, so when the test runtime
/// drops at end of test, those tasks shut down with it. Sharing one
/// client across multiple per-test runtimes deadlocks once the first
/// runtime drops its tasks.
pub fn client() -> Arc<HttpClient> {
    Arc::new(HttpClient::new(
        reqwest::Client::new(),
        Some(base_url().to_string()),
        None::<String>,
        None::<String>,
        None::<String>,
        None::<String>,
        None::<String>,
        None::<String>,
        None,
        None::<String>,
        None::<String>,
        None::<String>,
        None::<String>,
        None::<String>,
        None::<String>,
    ))
}

/// The shared server's base URL, e.g. `http://127.0.0.1:53241`.
/// Cached per process so the env-var lookup and panic check only run
/// once.
pub fn base_url() -> &'static str {
    use std::sync::OnceLock;
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(resolve_base_url)
}
