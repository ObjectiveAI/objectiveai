//! Connects to the shared test api server — self-resolving, no
//! harness required.
//!
//! All integration test binaries (this suite's AND every other suite
//! in the repo) share **one** api server instance: each consumer
//! runs `objectiveai api spawn` against the repo's committed
//! `.objectiveai` test root, and the api lockfile singleton
//! guarantees exactly one server ever materializes — whoever asks
//! first spawns it (the bin entry points at the pre-built
//! `target/debug` binary from `test-build.sh`, which `test.sh` runs
//! up front), everyone else gets the already-published URL back. The URL itself is read from the `api`
//! lockfile's published contents. Cross-binary state isolation is
//! handled inside the api (per-request) and inside the tests
//! (per-test states), not at the OS process level.

use std::path::PathBuf;
use std::sync::Arc;

use objectiveai_sdk::HttpClient;

/// `<repo>/.objectiveai` — the committed shared test OBJECTIVEAI_DIR.
fn objectiveai_dir() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .join(".objectiveai")
}

/// Spawn-or-discover the shared api server and return its base URL.
///
/// `api spawn` is idempotent and cheap when the server is already up
/// (it returns the lock-published URL without spawning); afterwards
/// the URL is read from the lockfile at `<dir>/bin/locks` key `api`.
/// The lockfile read needs a tokio reactor and this may be called
/// from inside a test runtime, so it runs on a dedicated thread with
/// its own current-thread runtime.
fn resolve_base_url() -> String {
    let dir = objectiveai_dir();
    let shim = dir.join("bin").join(if cfg!(windows) {
        "objectiveai.exe"
    } else {
        "objectiveai"
    });
    // Capture stdout: the cli reports errors as JSON lines on stdout,
    // so on failure the captured bytes ARE the diagnosis.
    let output = std::process::Command::new(&shim)
        .args(["api", "spawn"])
        .env("OBJECTIVEAI_DIR", &dir)
        .output()
        .expect("run the objectiveai shim for `api spawn`");
    assert!(
        output.status.success(),
        "`objectiveai api spawn` failed ({}): {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
    );

    let locks_dir = dir.join("bin").join("locks");
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build lockfile-read runtime")
            .block_on(objectiveai_sdk::lockfile::try_read(&locks_dir, "api"))
    })
    .join()
    .expect("lockfile-read thread")
    .expect("read api lockfile")
    .expect("api lock published a URL")
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
