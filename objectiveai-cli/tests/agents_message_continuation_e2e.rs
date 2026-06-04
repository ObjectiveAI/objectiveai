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
//!
//! Driven through the SDK `BinaryExecutor` rather than hand-rolled
//! argv. The test still pokes at the cli's on-disk continuation logs
//! to verify byte-level propagation — that path-walking is the
//! load-bearing part of the assertion, independent of the executor.

mod cli_test_util;

use std::path::Path;
use std::time::{Duration, Instant};

use objectiveai_sdk::cli::command::agents::message::{
    Request as MessageRequest, RequestMessage, Response as MessageResponse,
};
use objectiveai_sdk::cli::command::agents::spawn::{
    AgentSpec, Request as SpawnRequest, RequestPrompt, ResponseItem as SpawnResponseItem,
};
use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use serde_json::Value;

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

    let base_dir = cli_test_util::test_base_dir();
    let base_dir = base_dir.as_path();

    let executor = cli_test_util::executor_with_base_dir(base_dir);

    // ── 1. Spawn a mock agent ────────────────────────────────────
    let spawn_request = SpawnRequest { path_type: objectiveai_sdk::cli::command::agents::spawn::Path::AgentsSpawn,
        prompt: RequestPrompt::Simple("first turn".to_string()),
        agent: AgentSpec::Resolved(
            serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
                serde_json::json!({"upstream":"mock","output_mode":"instruction"}),
            )
            .expect("inline mock agent must deserialize"),
        ),
        seed: Some(42),
        dangerous_advanced: None,
        jq: None,
    };
    let spawn_items: Vec<SpawnResponseItem> =
        cli_test_util::collect_stream(&executor, spawn_request).await;
    let spawn_id = spawn_items
        .iter()
        .find_map(|item| match item {
            SpawnResponseItem::Chunk(chunk) => {
                if chunk.agent_instance_hierarchy.is_empty() {
                    None
                } else {
                    Some(chunk.agent_instance_hierarchy.clone())
                }
            }
            SpawnResponseItem::Id(_) => None,
        })
        .expect("agents spawn must emit a Chunk with agent_instance_hierarchy");

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
    let socket_path = base_dir.join("pipes/cli").join(&spawn_id).join("socket");
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
    // Split the chunk's full lineage into (parent, instance) for the
    // new two-field `MessageRequest` shape.
    let (parent, instance) = spawn_id
        .rsplit_once('/')
        .map(|(p, i)| (Some(p.to_string()), i.to_string()))
        .unwrap_or_else(|| (None, spawn_id.clone()));
    let message_request = MessageRequest {
        path_type: objectiveai_sdk::cli::command::agents::message::Path::AgentsMessage,
        parent_agent_instance_hierarchy: parent,
        agent_instance: instance,
        message: RequestMessage::Simple("follow up".to_string()),
        seed: Some(42),
        jq: None,
    };
    let response: MessageResponse =
        cli_test_util::execute_one(&executor, message_request).await;
    let new_response_id = match response {
        MessageResponse::Queued { response_id, .. } => response_id,
        MessageResponse::Delivered { .. } => panic!(
            "agents message must take the fallback path (Queued), got Delivered — the cli \
             stream from the spawn turn never tore down cleanly"
        ),
    };
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
    // case we caught it mid-write. `.continuation` is a
    // `LogReference { type, path }` pointing at a sibling `.txt` file
    // that holds the raw continuation token — same on-disk format
    // every other request-side referenced leaf uses. Follow the path
    // and read that file's content (raw bytes, NOT JSON-quoted) so
    // the comparison below is `token vs token`.
    let request_cont_raw =
        read_referenced_continuation(&request_summary_path, base_dir).await;

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

/// Poll the request-summary JSON, follow `.continuation.path` to the
/// referenced leaf file, and return its raw contents (un-JSON-quoted).
async fn read_referenced_continuation(
    request_summary_path: &Path,
    base_dir: &Path,
) -> String {
    let mut last_err: Option<String> = None;
    let mut value: Option<String> = None;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        match std::fs::read_to_string(request_summary_path) {
            Ok(s) if !s.is_empty() => match serde_json::from_str::<Value>(&s) {
                Ok(v) => {
                    let path = v
                        .get("continuation")
                        .and_then(|c| c.get("path"))
                        .and_then(|p| p.as_str());
                    match path {
                        Some(p) => {
                            let leaf = base_dir.join("logs").join(p);
                            match std::fs::read_to_string(&leaf) {
                                Ok(token) if !token.is_empty() => {
                                    value = Some(token);
                                    break;
                                }
                                Ok(_) => {
                                    last_err =
                                        Some("continuation leaf empty".to_string());
                                }
                                Err(e) => {
                                    last_err = Some(format!(
                                        "read continuation leaf {}: {e}",
                                        leaf.display()
                                    ));
                                }
                            }
                        }
                        None => {
                            last_err = Some("no .continuation.path field".to_string());
                        }
                    }
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
}
