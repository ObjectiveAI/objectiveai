//! Connects to the shared test api server.
//!
//! The server's base URL is supplied by the test harness via the
//! `OBJECTIVEAI_ADDRESS` environment variable — the same var the
//! objectiveai client normally reads for its address. The root
//! `test.sh` runs `api spawn` up front and exports its published URL.
//! These tests no longer spawn or discover a server themselves.

use std::sync::Arc;

use objectiveai_sdk::HttpClient;

/// The shared server's base URL, read from `OBJECTIVEAI_ADDRESS`
/// (e.g. `http://127.0.0.1:53241`).
fn resolve_base_url() -> String {
    std::env::var("OBJECTIVEAI_ADDRESS").expect(
        "OBJECTIVEAI_ADDRESS must be set to the running API server's base URL",
    )
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
