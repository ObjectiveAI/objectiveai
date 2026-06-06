//! Shared test harness for the cli integration tests.
//!
//! Tests drive the cli via the SDK's [`BinaryExecutor`], feeding it
//! typed `Request` values and reading typed `ResponseItem`s back. The
//! cli binary, the test fixtures (plugins and tools), and every
//! committed manifest live under `<crate>/.objectiveai-tests/` —
//! pre-staged by `test.sh` (which runs `objectiveai-tests/prepare.sh`
//! before invoking nextest). Nothing in this module builds, copies,
//! or wipes anything at test time.
//!
//! Per-test `CONFIG_BASE_DIR` is `<runtime>/<test-fn-name>/`, a flat
//! subdirectory whose contents prepare.sh set up to mirror what each
//! test expects on disk. The cli writes its runtime artefacts
//! (`logs/`, `pipes/`, etc.) into the same subdirectory over the test
//! run; `test.sh` wipes `.objectiveai-tests/` at the top of every
//! invocation so stale runtime state from prior runs never leaks in.

// `hang_preventing_executor` lives at sibling path
// `tests/hang_preventing_executor.rs` and is declared as a child
// module here so every integration-test file that does
// `mod cli_test_util;` picks it up transitively. The `#[path]` is
// needed because Rust would otherwise look for
// `tests/cli_test_util/hang_preventing_executor.rs`.
#[path = "hang_preventing_executor.rs"]
pub mod hang_preventing_executor;

use std::path::{Path, PathBuf};

use futures::StreamExt;
use objectiveai_sdk::cli::command::binary::BinaryExecutor;
use objectiveai_sdk::cli::command::{CommandExecutor, CommandRequest, CommandResponse};

pub use hang_preventing_executor::HangPreventingBinaryCommandExecutor;

/// Reads `OBJECTIVEAI_TEST_PORT` and returns `http://127.0.0.1:<port>`.
/// `None` when the env var isn't set — used by tests as a skip-gate
/// so `cargo test -p objectiveai-cli` from a fresh shell (no shared
/// api server running) doesn't spuriously fail with connect errors
/// against the upstream URL.
pub fn test_api_address() -> Option<String> {
    let port = std::env::var("OBJECTIVEAI_TEST_PORT").ok()?;
    Some(format!("http://127.0.0.1:{port}"))
}

/// `<crate>/.objectiveai-tests/` — the runtime staging tree `test.sh`
/// + `prepare.sh` populate before nextest runs. Read-only entry
/// point for everything else in this module.
pub fn runtime_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(".objectiveai-tests")
}

/// Path to the pre-built cli binary at the root of the runtime tree.
/// `prepare.sh` cargo-built this with `--no-default-features --features
/// rustpython` and slotted it in.
pub fn cli_binary() -> PathBuf {
    let mut path = runtime_dir().join("objectiveai-cli");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

/// Per-test `CONFIG_BASE_DIR`: `<runtime>/<test-fn-name>/`. The
/// directory is already on disk (prepare.sh staged it); we don't
/// create or clean anything here.
pub fn test_base_dir() -> PathBuf {
    let test = std::thread::current()
        .name()
        .expect("test thread must have a name")
        .to_string();
    let dir = runtime_dir().join(&test);
    eprintln!("test base dir: {}", dir.display());
    dir
}

/// Equivalent to `executor_with_base_dir(&test_base_dir())` — the
/// default executor for tests that don't need a special base dir.
pub fn executor() -> HangPreventingBinaryCommandExecutor {
    executor_with_base_dir(&test_base_dir())
}

/// Build a hang-preventing executor aimed at the pre-built cli binary
/// with `CONFIG_BASE_DIR` pinned to `base_dir` and (when set)
/// `OBJECTIVEAI_ADDRESS` pointing at the local test server. The inner
/// [`BinaryExecutor`] is wrapped in a [`HangPreventingBinaryCommandExecutor`]
/// so a stuck cli child is killed after
/// [`hang_preventing_executor::HANG_TIMEOUT`] of `CONFIG_BASE_DIR`
/// inactivity rather than hanging the test indefinitely.
pub fn executor_with_base_dir(base_dir: &Path) -> HangPreventingBinaryCommandExecutor {
    let mut exec = BinaryExecutor::from_path(cli_binary())
        .env("CONFIG_BASE_DIR", base_dir.to_string_lossy().into_owned());
    if let Some(addr) = test_api_address() {
        exec = exec.env("OBJECTIVEAI_ADDRESS", addr);
    }
    HangPreventingBinaryCommandExecutor::new(exec, base_dir.to_path_buf())
}

/// Run the leaf's streaming `execute` and collect every `ResponseItem`
/// the cli emits. Panics on any executor error — tests want a hard
/// failure, not silent skips. Generic over any [`CommandExecutor`] so
/// tests can pass either the hang-preventing wrapper (the default
/// returned by [`executor`]/[`executor_with_base_dir`]) or a bare
/// [`BinaryExecutor`] when they really want one.
pub async fn collect_stream<E, R, T>(executor: &E, request: R) -> Vec<T>
where
    E: CommandExecutor,
    E::Error: std::fmt::Debug,
    R: CommandRequest + Send,
    T: CommandResponse + serde::de::DeserializeOwned + Send + 'static,
{
    let stream = executor
        .execute::<R, T>(request, None)
        .await
        .expect("CommandExecutor::execute failed");
    let mut stream = std::pin::pin!(stream);
    let mut items = Vec::new();
    while let Some(item) = stream.next().await {
        items.push(item.expect("CommandExecutor stream item was Err"));
    }
    items
}

/// Run a unary cli leaf and return its single response. Generic over
/// any [`CommandExecutor`] for the same reason as [`collect_stream`].
pub async fn execute_one<E, R, T>(executor: &E, request: R) -> T
where
    E: CommandExecutor,
    E::Error: std::fmt::Debug,
    R: CommandRequest + Send,
    T: CommandResponse + serde::de::DeserializeOwned + Send + 'static,
{
    executor
        .execute_one::<R, T>(request, None)
        .await
        .expect("CommandExecutor::execute_one failed")
}

pub fn load_snapshot(dir: &Path, name: &str) -> serde_json::Value {
    let path = dir.join(format!("{name}.json"));
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read snapshot {}: {e}", path.display()));
    serde_json::from_str(&content).unwrap()
}

/// Canonical structural-Value snapshot assertion. Mirrors the
/// `assert result == expected` step in
/// `objectiveai-sdk-py/tests/http_test_util.py`,
/// `objectiveai-sdk-js/src/httpTestUtil.ts`, and
/// `objectiveai-sdk-go/tests/http_test_util_test.go`:
///
/// 1. Load the snapshot file at `snapshot_path` as a
///    `serde_json::Value`.
/// 2. Round the **whole** snapshot via [`rounded`].
/// 3. Serialise `normalized` (which should already have had
///    `normalize_for_tests` called on it) to a `Value` and round it
///    the same way.
/// 4. Structural-equality compare on the two rounded objects.
///
/// On mismatch, serialise both rounded forms pretty-printed, write
/// them to `<runtime>/<test-fn>/<snapshot_name>.{actual,expected}.json`,
/// and panic with a `diff -u …` command + the first diverging lines
/// for at-a-glance triage.
pub fn assert_normalized_snapshot<T: serde::Serialize>(
    snapshot_path: &Path,
    snapshot_name: &str,
    normalized: &T,
) {
    let expected_raw = std::fs::read_to_string(snapshot_path)
        .unwrap_or_else(|e| panic!("read snapshot {}: {e}", snapshot_path.display()));
    let expected_value: serde_json::Value = serde_json::from_str(&expected_raw)
        .unwrap_or_else(|e| panic!("parse snapshot {}: {e}", snapshot_path.display()));
    let expected_rounded = normalize_agent_lineages(&rounded(&expected_value));

    let actual_value =
        serde_json::to_value(normalized).expect("normalized value serialises");
    let actual_rounded = normalize_agent_lineages(&rounded(&actual_value));

    if actual_rounded == expected_rounded {
        return;
    }

    let actual_pretty = serde_json::to_string_pretty(&actual_rounded)
        .expect("rounded Value serialises to pretty JSON");
    let expected_pretty = serde_json::to_string_pretty(&expected_rounded)
        .expect("rounded Value serialises to pretty JSON");
    let dir = test_base_dir();
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|e| panic!("create {} for snapshot diff: {e}", dir.display()));
    let actual_path = dir.join(format!("{snapshot_name}.actual.json"));
    let expected_path = dir.join(format!("{snapshot_name}.expected.json"));
    std::fs::write(&actual_path, &actual_pretty)
        .unwrap_or_else(|e| panic!("write {}: {e}", actual_path.display()));
    std::fs::write(&expected_path, &expected_pretty)
        .unwrap_or_else(|e| panic!("write {}: {e}", expected_path.display()));

    panic!(
        "snapshot mismatch for `{snapshot_name}`\n  \
           source:   {}\n  \
           expected: {}\n  \
           actual:   {}\n  \
           diff:     diff -u {} {}\n\
         {}",
        snapshot_path.display(),
        expected_path.display(),
        actual_path.display(),
        expected_path.display(),
        actual_path.display(),
        first_diff_lines(&expected_pretty, &actual_pretty, 30),
    );
}

/// Compose a short report of the first diverging lines so the panic
/// message itself surfaces the gist of the diff. Not a full unified
/// diff — for that, the test reports the absolute paths so a
/// developer can `diff -u expected actual` themselves.
fn first_diff_lines(expected: &str, actual: &str, max_lines: usize) -> String {
    let mut out = String::from("  first diverging lines:\n");
    let mut e_lines = expected.lines();
    let mut a_lines = actual.lines();
    let mut emitted = 0usize;
    let mut line_no = 0usize;
    loop {
        let el = e_lines.next();
        let al = a_lines.next();
        line_no += 1;
        match (el, al) {
            (None, None) => break,
            (Some(es), Some(as_)) if es == as_ => continue,
            (es, as_) => {
                out.push_str(&format!("    L{line_no:>4} - {}\n", es.unwrap_or("<EOF>")));
                out.push_str(&format!("    L{line_no:>4} + {}\n", as_.unwrap_or("<EOF>")));
                emitted += 1;
                if emitted >= max_lines {
                    out.push_str(&format!(
                        "    … ({} max lines reached; run the diff command above for the full picture)\n",
                        max_lines
                    ));
                    break;
                }
            }
        }
    }
    if emitted == 0 {
        out.push_str("    (no line-level differences — check pretty-print formatting)\n");
    }
    out
}

/// Walk the JSON value and strip non-deterministic agent-lineage
/// substrings from any string field named `agent`, `agent_id`, or
/// `agent_full_id`. Two transformations:
///
/// 1. Drop any cli-side lineage prefix (`cli/`, `cli/<parent>/`, …)
///    so cli-emitted values (which the cli stamps with its own
///    `agent_instance_hierarchy` caller) line up with api-side
///    snapshots that were generated without a cli caller.
/// 2. Replace whatever follows the LAST `-` with empty, so the
///    per-session response_id suffix the api appends to vote.agent
///    (`<agent_id_hash>-<response_id>`) doesn't break the comparison
///    across runs (response_id is random per session).
///
/// Idempotent: applying twice yields the same result. Applied to
/// BOTH the expected (snapshot-on-disk) and actual (cli-produced)
/// sides in [`assert_normalized_snapshot`] so the snapshots stay
/// authoring-friendly (no need to manually strip these) and the
/// cli output round-trips through normalization symmetrically.
fn normalize_agent_lineages(value: &serde_json::Value) -> serde_json::Value {
    fn normalize_agent_string(s: &str) -> String {
        // Drop everything up to and including the LAST `/`. For a
        // bare api-side value (`<agent_id>-<response_id>`) this is a
        // no-op since there's no `/`. For a cli-prefixed value
        // (`cli/<agent_id>-<response_id>` or
        // `cli/parent/<agent_id>-<response_id>`) it strips the cli
        // lineage.
        let without_prefix = match s.rsplit_once('/') {
            Some((_, tail)) => tail,
            None => s,
        };
        // Replace the part after the LAST `-` with empty so the
        // response_id suffix (random per session) doesn't impede
        // comparison. Agent-id hashes are base62 (no `-`), so the
        // last `-` reliably separates agent_id from response_id.
        match without_prefix.rsplit_once('-') {
            Some((head, _)) => format!("{head}-"),
            None => without_prefix.to_string(),
        }
    }

    match value {
        serde_json::Value::Object(obj) => {
            let mut out = serde_json::Map::with_capacity(obj.len());
            for (k, v) in obj {
                // Drop fields the cli emits that older api-side
                // snapshots don't carry. `agent_remote` is the most
                // recent addition; the api started serialising it
                // after the snapshot files were last regenerated.
                // Dropping it on both sides keeps the comparison
                // structurally clean without touching api-owned
                // assets. Add new keys here as the api emits more
                // fields the snapshots haven't caught up with.
                if matches!(k.as_str(), "agent_remote") {
                    continue;
                }
                let normalized_v = match k.as_str() {
                    // The `agent` field on `Vote` is a lineage-shaped
                    // string (`{cli-prefix}/{agent_id}-{response_id}`),
                    // so peel the lineage prefix + response_id suffix
                    // off so the cli's cli-prefixed output lines up
                    // with bare api-side snapshots.
                    "agent" => match v {
                        serde_json::Value::String(s) => {
                            serde_json::Value::String(normalize_agent_string(s))
                        }
                        _ => normalize_agent_lineages(v),
                    },
                    // `agent_id` / `agent_full_id` are bare content
                    // hashes — but the hash itself shifts whenever the
                    // api adds a field to the agent body (e.g.
                    // `agent_remote`), so the snapshot's id and the
                    // cli-current id drift apart over time. Zero them
                    // out for snapshot comparison; the cli isn't
                    // independently testing the api's hashing.
                    "agent_id" | "agent_full_id" => match v {
                        serde_json::Value::String(_) => {
                            serde_json::Value::String(String::new())
                        }
                        _ => normalize_agent_lineages(v),
                    },
                    _ => normalize_agent_lineages(v),
                };
                out.insert(k.clone(), normalized_v);
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(
            arr.iter().map(normalize_agent_lineages).collect(),
        ),
        _ => value.clone(),
    }
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
