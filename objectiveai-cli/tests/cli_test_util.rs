//! Shared test harness for the cli integration tests.
//!
//! Tests drive the cli via the SDK's [`BinaryExecutor`], feeding it
//! typed `Request` values and reading typed `ResponseItem`s back. The
//! executor spawns the cli binary that `cli_binary()` builds (one
//! cargo build per test run, shared via a `Once`).
//!
//! Per-test scratch dirs use [`executor_with_base_dir`] — the
//! `CONFIG_BASE_DIR` env var is attached to the spawned child rather
//! than the test runner, so parallel tests with independent scratch
//! dirs don't race on a shared process-level env.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

use futures::StreamExt;
use objectiveai_sdk::cli::command::command_executor::binary::BinaryExecutor;
use objectiveai_sdk::cli::command::{CommandExecutor, CommandRequest};

static BUILD_ONCE: Once = Once::new();

/// Reads `OBJECTIVEAI_TEST_PORT` and returns `http://127.0.0.1:<port>`.
/// `None` when the env var isn't set — used by the snapshot tests as a
/// skip-gate so `cargo test -p objectiveai-cli` from a fresh shell
/// (no shared api server running) doesn't spuriously fail with connect
/// errors against the upstream URL.
pub fn test_api_address() -> Option<String> {
    let port = std::env::var("OBJECTIVEAI_TEST_PORT").ok()?;
    Some(format!("http://127.0.0.1:{port}"))
}

pub fn test_target_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/test-cli")
}

pub fn cli_binary() -> PathBuf {
    let target_dir = test_target_dir();
    let mut path = target_dir.join("debug/objectiveai-cli");
    if cfg!(windows) {
        path.set_extension("exe");
    }

    BUILD_ONCE.call_once(|| {
        let status = Command::new("cargo")
            .args([
                "build",
                "-p",
                "objectiveai-cli",
                "--no-default-features",
                "--features",
                "rustpython",
                "--target-dir",
                target_dir.to_str().unwrap(),
            ])
            .status()
            .expect("failed to run cargo build");
        assert!(status.success(), "cargo build failed");
    });

    path
}

/// CONFIG_BASE_DIR for the CLI under test.
///
/// Scoped to `tests/.objectiveai` so everything the CLI creates at runtime
/// (logs, cached function repos, filesystem config) lives under a single
/// gitignored directory that `test.sh` wipes on exit.
pub fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(".objectiveai")
}

/// Build a [`BinaryExecutor`] aimed at the test-built cli binary with
/// `CONFIG_BASE_DIR` set to the shared `tests/.objectiveai` scratch dir
/// and (when set) `OBJECTIVEAI_ADDRESS` pointing at the local test
/// server. Every integration test in the suite must go through this
/// helper so the env plumbing stays consistent.
pub fn executor() -> BinaryExecutor {
    executor_with_base_dir(&tests_dir())
}

/// Variant of [`executor`] that pins `CONFIG_BASE_DIR` to a
/// caller-supplied directory rather than the shared scratch dir. Used
/// by the two `agents_*_continuation` e2e tests, which need a fresh
/// `tempfile::tempdir()` per run so the spawn doesn't trip on stale
/// state.
pub fn executor_with_base_dir(base_dir: &Path) -> BinaryExecutor {
    let mut exec = BinaryExecutor::from_path(cli_binary())
        .env("CONFIG_BASE_DIR", base_dir.to_string_lossy().into_owned());
    if let Some(addr) = test_api_address() {
        exec = exec.env("OBJECTIVEAI_ADDRESS", addr);
    }
    exec
}

/// Run the leaf's streaming `execute` and collect every `ResponseItem`
/// the cli emits. Panics on any executor error — tests want a hard
/// failure, not silent skips.
pub async fn collect_stream<R, T>(executor: &BinaryExecutor, request: R) -> Vec<T>
where
    R: CommandRequest + Send,
    T: serde::de::DeserializeOwned + Send + 'static,
{
    let stream = executor
        .execute::<R, T>(request)
        .await
        .expect("BinaryExecutor::execute failed");
    let mut stream = std::pin::pin!(stream);
    let mut items = Vec::new();
    while let Some(item) = stream.next().await {
        items.push(item.expect("BinaryExecutor stream item was Err"));
    }
    items
}

/// Run a unary cli leaf and return its single response.
pub async fn execute_one<R, T>(executor: &BinaryExecutor, request: R) -> T
where
    R: CommandRequest + Send,
    T: serde::de::DeserializeOwned + Send + 'static,
{
    executor
        .execute_one::<R, T>(request)
        .await
        .expect("BinaryExecutor::execute_one failed")
}

pub fn load_snapshot(dir: &Path, name: &str) -> serde_json::Value {
    let path = dir.join(format!("{name}.json"));
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read snapshot {}: {e}", path.display()));
    serde_json::from_str(&content).unwrap()
}

/// Round floats to 8 significant figures to match cross-language comparison.
pub fn rounded(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                let s12 = format!("{:.12e}", f);
                let f12: f64 = s12.parse().unwrap_or(f);
                let s8 = format!("{:.8e}", f12);
                let f8: f64 = s8.parse().unwrap_or(f12);
                serde_json::Value::Number(
                    serde_json::Number::from_f64(f8).unwrap_or_else(|| n.clone()),
                )
            } else {
                value.clone()
            }
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(rounded).collect())
        }
        serde_json::Value::Object(obj) => {
            serde_json::Value::Object(obj.iter().map(|(k, v)| (k.clone(), rounded(v))).collect())
        }
        _ => value.clone(),
    }
}
