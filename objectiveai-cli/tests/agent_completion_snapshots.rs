mod cli_test_util;

use std::path::{Path, PathBuf};
use std::process::Command;

fn snapshots_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../objectiveai-api/assets/agent/completions/client_tests")
}

/// Extract the last assistant message content from a snapshot.
fn extract_assistant_content(snapshot: &serde_json::Value) -> String {
    let messages = snapshot["messages"].as_array().expect("snapshot should have messages array");
    messages.iter().rev()
        .find_map(|msg| {
            if msg["role"].as_str() == Some("assistant") {
                msg.get("content").and_then(|c| c.as_str()).map(String::from)
            } else {
                None
            }
        })
        .unwrap_or_default()
}

/// Run the CLI and return raw stdout as a string (not JSON-parsed).
/// Strips any "Logs ID:" prefix lines from the output.
fn run_cli_text(args: &[&str]) -> String {
    let mut cmd = Command::new(cli_test_util::cli_binary());
    cmd.env("CONFIG_BASE_DIR", cli_test_util::tests_dir());
    cmd.args(args);

    let output = cmd.output().expect("failed to execute CLI binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        panic!(
            "CLI exited with {}\nargs: {:?}\nstdout: {stdout}\nstderr: {stderr}",
            output.status, args
        );
    }

    stdout.lines()
        .filter(|line| !line.starts_with("Logs ID: "))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

macro_rules! snapshot_test {
    ($name:ident, $snapshot:expr, $messages_json:expr, $agent_name:expr, $seed:expr $(, $extra_args:expr)*) => {
        #[test]
        fn $name() {
            let seed_str = $seed.to_string();
            let agent_str = format!("remote=mock,name={}", $agent_name);
            let instructions_id = cli_test_util::instructions_id(
                cli_test_util::InstructionsScope::AgentCompletions,
            );
            let mut args = vec![
                "agents", "completions", "create", "standard",
                "--agent", &agent_str,
                "--messages-inline", $messages_json,
                "--seed", &seed_str,
                "--instructions-id", instructions_id.as_str(),
            ];
            $(
                for arg in $extra_args {
                    args.push(arg);
                }
            )*

            let actual = run_cli_text(&args);

            let snapshot = cli_test_util::load_snapshot(&snapshots_dir(), $snapshot);
            let expected = extract_assistant_content(&snapshot);

            assert_eq!(actual, expected, "assistant content mismatch for {}", $snapshot);
        }
    };
}

snapshot_test!(
    test_basic_mock_agent_seed_42,
    "test_basic_mock_agent_seed_42",
    "[]",
    "instruction",
    42
);

snapshot_test!(
    test_with_developer_and_user_messages,
    "test_with_developer_and_user_messages",
    r#"[{"role":"developer","content":"You are a helpful assistant."},{"role":"user","content":"What is 2+2?"}]"#,
    "instruction",
    99
);

snapshot_test!(
    test_json_object_response_format,
    "test_json_object_response_format",
    "[]",
    "instruction",
    42,
    &["--response-format-json-object"]
);
