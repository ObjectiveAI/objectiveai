//! End-to-end test: a fixture plugin binary is copied into a temp
//! plugins dir and dispatched via `objectiveai-cli plugins <name> <args>`.
//! We assert the host's JSONL output contains the expected re-emitted
//! notification between the `begin` / `end` markers.

mod cli_test_util;

use serde_json::Value;
use std::process::Command;

#[test]
fn hello_plugin_dispatch_produces_expected_output() {
    let base = cli_test_util::test_base_dir();
    let cli = cli_test_util::cli_binary();
    let output = Command::new(cli)
        .env("CONFIG_BASE_DIR", &base)
        .args([
            "plugins",
            "run",
            "--owner",
            "objectiveai",
            "--name",
            "hello",
            "--version",
            "0.0.1",
            "--args",
            "[\"world\"]",
        ])
        .output()
        .expect("failed to run cli");

    assert!(
        output.status.success(),
        "cli exited {:?}; stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    // Each cli stdout line is the leaf `plugins::run::ResponseItem`
    // serialized at the wire. Every `cli/command` aggregator
    // `Response`/`ResponseItem` is `#[serde(untagged)]` (sdk commit
    // 39c3320e7), so the root `RunItem::Command(_)`,
    // `cli::command::ResponseItem`, and `plugins::ResponseItem`
    // wrappers all collapse. The leaf `plugins::run::ResponseItem`
    // is itself untagged with variants `Mcp { type: "mcp", url }`,
    // `Error { type: "error", ... }`, and `Notification(Value)` —
    // so a notification carrying the plugin's raw payload
    // `{"hello":"world"}` lands on the wire as bare `{"hello":"world"}`,
    // not wrapped in any `Plugins/Run/` envelope.
    let stdout = String::from_utf8(output.stdout).expect("cli stdout not utf-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        !lines.is_empty(),
        "expected at least one notification, got: {lines:?}"
    );

    let hello_count = lines
        .iter()
        .filter(|line| {
            let Ok(v) = serde_json::from_str::<Value>(line) else {
                return false;
            };
            v.pointer("/hello") == Some(&Value::String("world".into()))
        })
        .count();
    assert_eq!(
        hello_count, 1,
        "expected exactly one hello/world notification in {lines:?}"
    );

}
