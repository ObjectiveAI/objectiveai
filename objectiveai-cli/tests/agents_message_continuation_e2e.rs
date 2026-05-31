//! `agents spawn` → wait for cli-stream to finish → `agents message`
//! takes the continuation-fallback path → assert the second turn's
//! request continuation byte-equals the first turn's response
//! continuation.
//!
//! The point of the test is to lock in *response*-side continuation
//! propagation. Reverting the SDK fix that made `read_latest_continuation`
//! read the response-side `.json` would surface as a panic at the final
//! `assert_eq!` (the file the request-side producer writes would never
//! end up holding the original turn's continuation).

mod cli_test_util;

use std::path::Path;
use std::process::Command;
use std::sync::Once;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// `cli_test_util::cli_binary` only builds `objectiveai-cli`. Streaming
/// flows (every spawn / continuation completion) re-exec
/// `objectiveai-cli-stream` next to the cli, so this test needs that
/// binary present in the same `target/test-cli/debug/` dir. Build it
/// once per test-binary lifetime.
static BUILD_CLI_STREAM_ONCE: Once = Once::new();

fn ensure_cli_stream_built() {
    BUILD_CLI_STREAM_ONCE.call_once(|| {
        let target_dir = cli_test_util::test_target_dir();
        let status = Command::new("cargo")
            .args([
                "build",
                "-p",
                "objectiveai-cli-stream",
                "--target-dir",
                target_dir.to_str().unwrap(),
            ])
            .status()
            .expect("failed to spawn cargo build for objectiveai-cli-stream");
        assert!(status.success(), "cargo build of objectiveai-cli-stream failed");
    });
}

/// `cli_test_util::cli_command` pins `CONFIG_BASE_DIR` to the shared
/// `tests/.objectiveai` scratch dir; this test needs a fresh tempdir
/// per run so the spawn doesn't trip on stale state. Same env plumbing
/// otherwise.
fn cli_command_with_base_dir(base_dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(cli_test_util::cli_binary());
    cmd.env("CONFIG_BASE_DIR", base_dir);
    if let Some(addr) = cli_test_util::test_api_address() {
        cmd.env("OBJECTIVEAI_ADDRESS", addr);
    }
    cmd.args(args);
    cmd
}

/// Spawn cli, panic on non-zero exit, return every JSONL-parsed
/// stdout line in order. Unlike `cli_test_util::run_cli` we keep the
/// full stream — the test dispatches on the `value.kind`
/// discriminator and needs the typed Spawned / MessageQueued
/// notifications, not the unwrapped last line.
fn run_cli_with_base_dir(base_dir: &Path, args: &[&str]) -> Vec<Value> {
    let output = cli_command_with_base_dir(base_dir, args)
        .output()
        .expect("failed to execute cli binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        panic!(
            "cli exited with {}\nargs: {args:?}\nstdout: {stdout}\nstderr: {stderr}",
            output.status,
        );
    }
    stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
        .collect()
}

/// Sleep-poll `pred` every 50ms until it returns true, up to `timeout`.
async fn poll_until<F: Fn() -> bool>(timeout: Duration, pred: F) -> Result<(), ()> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if pred() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Err(())
}

#[tokio::test]
async fn spawn_then_message_propagates_response_continuation() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!(
            "OBJECTIVEAI_TEST_PORT not set — skipping spawn_then_message_propagates_response_continuation"
        );
        return;
    }

    ensure_cli_stream_built();

    let tmp = tempfile::tempdir().expect("tempdir");
    let base_dir = tmp.path();

    // ── 1. Spawn a mock agent ────────────────────────────────────
    let spawn_lines = run_cli_with_base_dir(
        base_dir,
        &[
            "agents",
            "spawn",
            "--agent-inline",
            r#"{"upstream":"mock","output_mode":"instruction"}"#,
            "--simple",
            "first turn",
            "--seed",
            "42",
        ],
    );
    let spawned = spawn_lines
        .iter()
        .find(|l| l.pointer("/value/kind") == Some(&json!("spawned")))
        .expect("agents spawn must emit a Spawned notification");
    let spawn_id = spawned
        .pointer("/value/agent_id")
        .and_then(|v| v.as_str())
        .expect("Spawned.agent_id")
        .to_string();

    // ── 2. Wait for cli-stream to fully finish ───────────────────
    //
    // "Finished" = the response continuation file landed AND the
    // per-agent socket file is gone (cli-stream unlinks on exit).
    // If we raced this check the next `agents message` invocation
    // could hit the live path instead of the fallback we want to
    // exercise.
    let response_cont_path = base_dir
        .join("logs/agents/completions/response/continuation")
        .join(format!("{spawn_id}.json"));
    let socket_path = base_dir
        .join("pipes/cli")
        .join(&spawn_id)
        .join("socket");
    poll_until(Duration::from_secs(30), || {
        response_cont_path.exists() && !socket_path.exists()
    })
    .await
    .expect("cli-stream did not produce a response continuation + tear down its socket in 30s");

    // ── 3. Capture the original response continuation ───────────
    let response_cont_raw: String = serde_json::from_slice(
        &std::fs::read(&response_cont_path).expect("read response continuation"),
    )
    .expect("response continuation is JSON-quoted string");

    // ── 4. Message the agent ─────────────────────────────────────
    let msg_lines = run_cli_with_base_dir(
        base_dir,
        &[
            "agents",
            "message",
            &spawn_id,
            "--simple",
            "follow up",
            "--seed",
            "42",
        ],
    );
    let queued = msg_lines
        .iter()
        .find(|l| l.pointer("/value/kind") == Some(&json!("message_queued")))
        .expect("agents message must emit MessageQueued on the fallback path");

    let new_response_id = queued
        .pointer("/value/response_id")
        .and_then(|v| v.as_str())
        .expect("MessageQueued.response_id")
        .to_string();
    let echoed_agent_id = queued
        .pointer("/value/agent_id")
        .and_then(|v| v.as_str())
        .expect("MessageQueued.agent_id");
    assert_eq!(
        echoed_agent_id,
        format!("cli/{spawn_id}"),
        "MessageQueued.agent_id should be the full lineage form — \
         caller (`cli` by default) glued onto the spawn's chunk.id, \
         matching what the writer stamps into `messages.agent_id`"
    );
    // Continuations from the api server reuse the original chunk.id
    // as the new turn's response_id (the agent's stable lineage id
    // is the same across turns). So new_response_id == spawn_id is
    // expected — no assertion that they differ.

    // ── 5. Wait for the new turn's request summary JSON ──────────
    //
    // cli-stream serializes the second turn's whole
    // `AgentCompletionCreateParams` blob to
    // `agents/completions/request/<new_id>.json`. The `continuation`
    // field stays inline on that JSON — same on-disk file the spawn
    // overwrote with turn 2's params.
    let request_summary_path = base_dir
        .join("logs/agents/completions/request")
        .join(format!("{new_response_id}.json"));
    poll_until(Duration::from_secs(30), || request_summary_path.exists())
        .await
        .expect("second turn's request summary .json did not land in 30s");

    // Re-poll briefly until the file is non-empty / parseable in
    // case we caught it mid-write.
    let request_cont_raw = {
        let mut last_err = None;
        let mut value: Option<String> = None;
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(10) {
            match std::fs::read_to_string(&request_summary_path) {
                Ok(s) if !s.is_empty() => match serde_json::from_str::<Value>(&s) {
                    Ok(v) => {
                        if let Some(c) = v.get("continuation").and_then(|c| c.as_str()) {
                            value = Some(c.to_string());
                            break;
                        }
                        last_err = Some("no .continuation field".to_string());
                    }
                    Err(e) => last_err = Some(format!("parse: {e}")),
                },
                _ => last_err = Some("empty / unreadable".to_string()),
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        value.unwrap_or_else(|| {
            panic!(
                "did not find non-empty .continuation in {} after 10s: {:?}",
                request_summary_path.display(),
                last_err
            )
        })
    };

    // ── 6. The smoking gun ──────────────────────────────────────
    //
    // The second turn's request `.continuation` field must byte-equal
    // the first turn's RESPONSE-side continuation. If the SDK had
    // read the request-side continuation by mistake (the bug we
    // just fixed), this assertion would fail because that file
    // didn't exist for the spawn turn and the fallback would have
    // errored out before reaching here.
    assert_eq!(
        request_cont_raw, response_cont_raw,
        "second turn's request continuation must equal first turn's response continuation",
    );
}
