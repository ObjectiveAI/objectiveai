//! End-to-end snapshot tests: fixture tool binaries are placed in a
//! temp tools dir and dispatched via `objectiveai-cli tools <name>`.
//! We assert on a deterministic summary (per-stream stdout/stderr
//! arrays + exit code) via insta snapshots.

mod cli_test_util;

use std::process::Command;

#[derive(serde::Serialize)]
struct Summary {
    exit_code: i32,
    stdout: Vec<String>,
    stderr: Vec<String>,
}

/// Spawn the cli (shared home + this test's state), args `tools run <name>`,
/// then bucket every `tools::run::ResponseItem` into its originating
/// stream. Order is preserved within each bucket; cross-bucket
/// ordering is dropped (the dispatch drains both streams
/// concurrently via `tokio_stream::StreamExt::merge`).
///
/// Each cli stdout line is the leaf `tools::run::ResponseItem`
/// serialized at the wire. Every `cli/command` aggregator
/// `Response`/`ResponseItem` is `#[serde(untagged)]` (sdk commit
/// 39c3320e7), so the `RunItem::Command(_)`, `cli::command::ResponseItem`,
/// and `tools::ResponseItem` wrappers all collapse and the wire is
/// just the leaf:
///   - `Stdout(String)` → a bare JSON string, e.g. `"hello, world"`
///   - `Stderr(cli::Error)` → the error struct directly, e.g.
///     `{"type":"error","message":"...", ...}`
/// — no `Tools/Run` wrapper to navigate.
fn run_and_summarize(name: &str) -> Summary {
    let cli = cli_test_util::cli_binary();
    let output = Command::new(cli)
        .env("OBJECTIVEAI_DIR", cli_test_util::objectiveai_dir())
        .env("OBJECTIVEAI_STATE", cli_test_util::test_state_name())
        .args([
            "tools", "run", "--owner", "objectiveai", "--name", name, "--version",
            "0.0.1",
        ])
        .output()
        .expect("failed to run cli");

    let stdout_text = String::from_utf8(output.stdout).expect("cli stdout not utf-8");
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    for line in stdout_text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(text) = v.as_str() {
            stdout_lines.push(text.to_string());
        } else if let Some(msg) = v.pointer("/message").and_then(|m| m.as_str()) {
            stderr_lines.push(msg.to_string());
        }
    }

    Summary {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: stdout_lines,
        stderr: stderr_lines,
    }
}

#[test]
fn hello_tool_dispatch_snapshot() {
    let _state_dir = cli_test_util::test_base_dir();
    let summary = run_and_summarize("hello");
    insta::assert_json_snapshot!(summary);
}

#[test]
fn error_tool_dispatch_snapshot() {
    let _state_dir = cli_test_util::test_base_dir();
    let summary = run_and_summarize("error");
    insta::assert_json_snapshot!(summary);
}
