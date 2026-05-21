use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Once, OnceLock};

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

/// Build a `Command` for the cli binary with `CONFIG_BASE_DIR` set to
/// the per-test scratch dir and, when `OBJECTIVEAI_TEST_PORT` is set,
/// `OBJECTIVEAI_ADDRESS` pointing at the local test server. Every cli
/// invocation in the test suite must go through this helper so the
/// env plumbing stays consistent.
pub fn cli_command(args: &[&str]) -> Command {
    let mut cmd = Command::new(cli_binary());
    cmd.env("CONFIG_BASE_DIR", tests_dir());
    if let Some(addr) = test_api_address() {
        cmd.env("OBJECTIVEAI_ADDRESS", addr);
    }
    cmd.args(args);
    cmd
}

/// Issue an Instructions ID for the given scope (e.g. "agents", "functions",
/// "functions-inventions-recursive") via the corresponding `instructions get`
/// command. The CLI now requires `--instructions-id <ID>` on every `create`
/// streaming command; tests run that subcommand once per scope and cache the
/// returned id.
pub fn instructions_id(scope: InstructionsScope) -> &'static String {
    let cell = scope.cell();
    cell.get_or_init(|| {
        let output = cli_command(scope.get_args())
            .output()
            .expect("failed to execute CLI binary");
        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!(
                "CLI {:?} exited with {}\nstdout: {stdout}\nstderr: {stderr}",
                scope.get_args(), output.status,
            );
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        // The CLI wraps its output in JSONL: `{begin}` / `{notification, value: {instructions: "...\n\n Instructions ID: <id>"}}` / `{end}`.
        // Find the notification and pluck the id out of the embedded markdown.
        let needle = "Instructions ID: ";
        let idx = stdout.find(needle).unwrap_or_else(|| {
            panic!("`Instructions ID: <id>` not found in output: {stdout}")
        });
        stdout[idx + needle.len()..]
            .split(|c: char| c.is_whitespace() || c == '"' || c == '\\')
            .next()
            .unwrap_or_else(|| panic!("empty Instructions ID in output: {stdout}"))
            .to_string()
    })
}

#[derive(Clone, Copy)]
pub enum InstructionsScope {
    AgentCompletions,
    FunctionExecutions,
    FunctionInventionsRecursive,
}

impl InstructionsScope {
    fn cell(self) -> &'static OnceLock<String> {
        match self {
            Self::AgentCompletions => {
                static CELL: OnceLock<String> = OnceLock::new();
                &CELL
            }
            Self::FunctionExecutions => {
                static CELL: OnceLock<String> = OnceLock::new();
                &CELL
            }
            Self::FunctionInventionsRecursive => {
                static CELL: OnceLock<String> = OnceLock::new();
                &CELL
            }
        }
    }

    fn get_args(self) -> &'static [&'static str] {
        match self {
            Self::AgentCompletions => &["agents", "completions", "instructions", "get"],
            Self::FunctionExecutions => &["functions", "executions", "instructions", "get"],
            Self::FunctionInventionsRecursive => &[
                "functions", "inventions", "recursive", "instructions", "get",
            ],
        }
    }
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
                "build", "-p", "objectiveai-cli",
                "--no-default-features", "--features", "rustpython",
                "--target-dir", target_dir.to_str().unwrap(),
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

/// Run a CLI command and return the last data-bearing JSONL
/// notification's `value`. The CLI wraps output in
/// `{begin}` / `{notification, value: {log_stream_ready: ...}}` /
/// `{notification, value: <payload>}` / `{end}`; tests want the
/// `<payload>` line, so we skip control markers and `log_stream_ready`
/// stubs and return the last remaining notification's value.
pub fn run_cli(args: &[&str]) -> serde_json::Value {
    let output = cli_command(args)
        .output()
        .expect("failed to execute CLI binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        panic!(
            "CLI exited with {}\nargs: {:?}\nstdout: {stdout}\nstderr: {stderr}",
            output.status, args
        );
    }

    let mut data_values: Vec<serde_json::Value> = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("Logs ID:") {
            continue;
        }
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };
        if parsed.get("type").and_then(|t| t.as_str()) != Some("notification") {
            continue;
        }
        let Some(value) = parsed.get("value") else {
            continue;
        };
        // Skip the `log_stream_ready` stub notification; tests want the
        // subsequent data-bearing notifications only.
        if value.as_object()
            .map(|o| o.len() == 1 && o.contains_key("log_stream_ready"))
            .unwrap_or(false)
        {
            continue;
        }
        data_values.push(value.clone());
    }
    if data_values.is_empty() {
        panic!("no data notification in CLI output:\n{stdout}");
    }
    // Tests written against the pre-JSONL CLI expect either the direct
    // payload (single notification) or an array of payloads (streaming
    // notifications). When there's exactly one notification with exactly
    // one object-typed wrapper key, descend through it — the cli wraps
    // command responses in keys like `execution`, `invention`, etc.
    if data_values.len() == 1 {
        let v = data_values.into_iter().next().unwrap();
        if let Some(obj) = v.as_object() {
            if obj.len() == 1 {
                let inner = obj.values().next().unwrap();
                if inner.is_object() || inner.is_array() {
                    return inner.clone();
                }
            }
        }
        return v;
    }
    serde_json::Value::Array(data_values)
}
