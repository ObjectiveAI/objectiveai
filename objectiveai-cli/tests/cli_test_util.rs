//! Shared test harness for the cli integration tests.
//!
//! Tests drive the cli via the SDK's [`BinaryExecutor`], feeding it
//! typed `Request` values and reading typed `ResponseItem`s back. The
//! executor spawns the cli binary that `cli_binary()` builds (one
//! cargo build per test run, shared via a `Once`).
//!
//! Every test's `CONFIG_BASE_DIR` lives under
//! `objectiveai-cli/.objectiveai-tests/<binary>/<test>/`, allocated
//! by [`test_base_dir`]. Each test binary clears its own
//! `<binary>/` subfolder on its first call (race-free `Once`-gated)
//! so re-runs always start clean — and crucially we DO NOT wipe on
//! drop, so the most recent run's logs survive long enough to
//! inspect. The path is echoed to stderr; pair with `cargo test --
//! --nocapture` when you need to find it.
//!
//! Carve-out: [`mcp_session_shared_dir`] returns a fixed
//! `.objectiveai-tests/_mcp_session/` shared with the tool-fixture
//! registry seeded by `test-seed-tool-fixtures.sh`. Only used by
//! the agents-continuation tool-session test.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

use futures::StreamExt;
use objectiveai_sdk::cli::command::binary::BinaryExecutor;
use objectiveai_sdk::cli::command::{CommandExecutor, CommandRequest, CommandResponse};

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

/// Absolute path to `objectiveai-cli/.objectiveai-tests/`. Creates
/// the dir + its `.gitignore` if either is missing. Idempotent and
/// safe to call from anywhere.
fn tests_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".objectiveai-tests");
    std::fs::create_dir_all(&root).expect("create .objectiveai-tests root");
    let gi = root.join(".gitignore");
    if !gi.exists() {
        std::fs::write(&gi, "*\n!.gitignore\n").expect("write .gitignore");
    }
    root
}

/// `<tests_root>/<binary-name>/`. On first call per test binary
/// process, clears the directory's contents (preserving the dir
/// itself) so this run starts clean. `Once`-gated; concurrent
/// callers wait for the clear before getting a path back.
fn binary_dir() -> PathBuf {
    static CLEAR_ONCE: Once = Once::new();
    let root = tests_root();
    let exe = std::env::current_exe().expect("current_exe");
    let stem = exe
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    // cargo names test binaries `<file_stem>-<hex>`. Strip the
    // trailing `-<hex>` to recover the stem; if the convention ever
    // changes, fall through to the full stem.
    let binary = stem
        .rsplit_once('-')
        .map(|(a, _)| a)
        .unwrap_or(stem)
        .to_string();
    let dir = root.join(&binary);
    CLEAR_ONCE.call_once(|| {
        if dir.exists() {
            for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let _ = std::fs::remove_dir_all(&p);
                } else {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }
        std::fs::create_dir_all(&dir).expect("create binary subdir");
    });
    dir
}

/// Fresh per-test `CONFIG_BASE_DIR` under
/// `.objectiveai-tests/<binary>/<test>/`. Echoes the resolved path
/// to stderr so `cargo test -- --nocapture` surfaces it for
/// debugging. Safe to call once per test fn; thread-safe.
///
/// **No cleanup on drop.** The previous run's contents survive
/// until the next run of the same test binary, which clears its
/// whole `<binary>/` subfolder via [`binary_dir`].
pub fn test_base_dir() -> PathBuf {
    let test = std::thread::current()
        .name()
        .map(sanitize_segment)
        .unwrap_or_else(|| format!("unnamed-{}", uuid::Uuid::new_v4()));
    let dir = binary_dir().join(&test);
    std::fs::create_dir_all(&dir).expect("create test subdir");
    eprintln!("test base dir: {}", dir.display());
    dir
}

/// Fixed `.objectiveai-tests/_mcp_session/` — the dedicated base
/// dir `test-seed-tool-fixtures.sh` lays the fixture `tools/`
/// registry into, and that every cli child stamps as its
/// `CONFIG_BASE_DIR` so its in-process objectiveai-mcp server
/// discovers the fixtures via `filesystem::Client::list_tools`.
/// Used only by `agents_continuation_tool_session_e2e` and the
/// fixture script. **Not** cleared by [`binary_dir`]'s `Once`; the
/// underscore-prefix puts it at the `tests_root` level, parallel to
/// the per-binary subfolders.
pub fn mcp_session_shared_dir() -> PathBuf {
    let dir = tests_root().join("_mcp_session");
    std::fs::create_dir_all(&dir).expect("create _mcp_session");
    dir
}

fn sanitize_segment(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Equivalent to `executor_with_base_dir(&test_base_dir())` — the
/// default executor for tests that don't need a special base dir.
pub fn executor() -> BinaryExecutor {
    executor_with_base_dir(&test_base_dir())
}

/// Build a [`BinaryExecutor`] aimed at the test-built cli binary
/// with `CONFIG_BASE_DIR` pinned to `base_dir` and (when set)
/// `OBJECTIVEAI_ADDRESS` pointing at the local test server.
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
    T: CommandResponse + serde::de::DeserializeOwned + Send + 'static,
{
    let stream = executor
        .execute::<R, T>(request, None)
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
    T: CommandResponse + serde::de::DeserializeOwned + Send + 'static,
{
    executor
        .execute_one::<R, T>(request, None)
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
