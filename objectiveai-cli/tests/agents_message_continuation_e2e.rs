//! `agents instances spawn` → wait for cli-stream to finish →
//! `agents instances message` takes the continuation-fallback path
//! → assert the second turn's request continuation byte-equals the
//! first turn's response continuation.
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

use objectiveai_sdk::cli::command::agents::instances::message::{
    MessageTarget, Request as MessageRequest,
    RequestDangerousAdvanced as MessageDangerousAdvanced, RequestMessage,
    ResponseItem as MessageResponseItem,
};
use objectiveai_sdk::cli::command::agents::instances::spawn::{
    AgentSpec, Request as SpawnRequest, RequestDangerousAdvanced, RequestPrompt,
    ResponseItem as SpawnResponseItem,
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
    // `dangerous_advanced.stream = true` keeps the parent cli
    // attached to its instance subprocess and forwards every
    // `AgentCompletionChunk` as `SpawnResponseItem::Chunk(_)`. We
    // need at least one Chunk to pull `agent_instance_hierarchy`
    // (full lineage, for the socket path) and `id` (leaf response
    // id, for the on-disk log file stems) off it. Without streaming
    // the parent cli detaches on `LogStreamReady` and emits only a
    // bare `Id(leaf)` — no Chunk, no `agent_instance_hierarchy`.
    let spawn_request = SpawnRequest { path_type: objectiveai_sdk::cli::command::agents::instances::spawn::Path::AgentsInstancesSpawn,
        agent_tag: None,
        prompt: RequestPrompt::Simple("first turn".to_string()),
        agent: AgentSpec::Resolved(
            serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
                serde_json::json!({"upstream":"mock","output_mode":"instruction"}),
            )
            .expect("inline mock agent must deserialize"),
        ),
        seed: Some(42),
        dangerous_advanced: Some(RequestDangerousAdvanced { stream: Some(true) }),
        jq: None,
    };
    let spawn_items: Vec<SpawnResponseItem> =
        cli_test_util::collect_stream(&executor, spawn_request).await;
    // Pull the leaf response id off the first non-empty Chunk.
    //
    // We deliberately ignore `chunk.agent_instance_hierarchy`: the API
    // emits it as `{caller}/{agent_full_id}-{response_id}` (the
    // api-side slot identifier), but the cli's on-disk filesystem
    // stores rows under `{caller}/{response_id_leaf}` (the
    // cli-side lineage). Using the api form for `agents instances
    // message` lookup would miss the cli's DB. The cli-side lineage is
    // `"{cli_caller}/{response_id_leaf}"`; we build it below from
    // `chunk.id` + the well-known cli caller prefix `"cli"`.
    let spawn_response_id = spawn_items
        .iter()
        .find_map(|item| match item {
            SpawnResponseItem::Chunk(chunk) => {
                if chunk.id.is_empty() {
                    None
                } else {
                    Some(chunk.id.clone())
                }
            }
            SpawnResponseItem::Id(_) => None,
        })
        .expect("agents instances spawn must emit a Chunk with non-empty id");
    // CLI-side lineage. The cli's `Config.agent_instance_hierarchy`
    // defaults to `"cli"` for caller-less invocations; combined with
    // the spawn's leaf response id this is what the cli's
    // `latest_continuation` lookup and the per-agent socket binder
    // both key on.
    let spawn_instance_hierarchy = format!("cli/{spawn_response_id}");

    // ── 2. Wait for cli-stream to fully finish ───────────────────
    //
    // "Finished" = the response continuation file landed AND the
    // per-agent socket file is gone (cli-stream unlinks on exit).
    // If we raced this check the next `agents instances message` invocation
    // could hit the live path instead of the fallback we want to
    // exercise.
    //
    // On-disk conventions (see
    // `objectiveai-cli/src/filesystem/logs/log_file_kind.rs`):
    //   continuation file:
    //     `logs/agents/completions/response/continuation/<leaf>.txt`
    //   per-agent socket:
    //     `pipes/<full-lineage>/socket`
    // Continuation stems on the LEAF response id; the socket stems
    // on the FULL `agent_instance_hierarchy` (which already starts
    // with `cli/`).
    let response_cont_path = base_dir
        .join("logs/agents/completions/response/continuation")
        .join(format!("{spawn_response_id}.txt"));
    let socket_path = base_dir
        .join("pipes")
        .join(&spawn_instance_hierarchy)
        .join("socket");
    poll_until(Duration::from_secs(30), || {
        response_cont_path.exists() && !socket_path.exists()
    })
    .await
    .expect("cli-stream did not produce a response continuation + tear down its socket in 30s");

    // ── 3. Capture the original response continuation ───────────
    // The on-disk file is the raw continuation token (base64-encoded
    // payload), written verbatim by `Client::read_text`'s mirror
    // writer — NOT a JSON-quoted string. Read it as bytes-as-utf8
    // and trim any trailing newline the writer added.
    let response_cont_raw = std::fs::read_to_string(&response_cont_path)
        .expect("read response continuation")
        .trim_end_matches('\n')
        .to_string();

    // ── 4. Message the agent ─────────────────────────────────────
    // Split the chunk's full lineage into (parent, instance) for the
    // new two-field `MessageRequest` shape.
    let (parent, instance) = spawn_instance_hierarchy
        .rsplit_once('/')
        .map(|(p, i)| (Some(p.to_string()), i.to_string()))
        .unwrap_or_else(|| (None, spawn_instance_hierarchy.clone()));
    // `dangerous_advanced.stream = Some(true)` keeps the parent cli
    // attached to the spawned instance runner — `collect_stream`
    // returning implies the runner exited, avoiding the leak nextest
    // would otherwise flag.
    let message_request = MessageRequest {
        path_type: objectiveai_sdk::cli::command::agents::instances::message::Path::AgentsInstancesMessage,
        target: MessageTarget::Direct {
            parent_agent_instance_hierarchy: parent,
            agent_instance: instance,
            agent_tag: None,
        },
        message: RequestMessage::Simple("follow up".to_string()),
        seed: Some(42),
        dangerous_advanced: Some(MessageDangerousAdvanced {
            stream: Some(true),
        }),
        jq: None,
    };
    let items: Vec<MessageResponseItem> =
        cli_test_util::collect_stream(&executor, message_request).await;
    // Item 0 carries the Queued envelope (or Delivered, which would
    // mean the cli stream from turn 1 never tore down).
    let new_response_id = match items.first() {
        Some(MessageResponseItem::Queued { response_id, .. }) => response_id.clone(),
        Some(MessageResponseItem::Delivered { .. }) => panic!(
            "agents instances message must take the fallback path (Queued), got Delivered — the cli \
             stream from the spawn turn never tore down cleanly"
        ),
        Some(MessageResponseItem::Chunk(_)) => panic!(
            "first stream item must be Queued/Delivered, got Chunk"
        ),
        None => panic!("agents instances message yielded no items"),
    };
    // Continuations from the api server reuse the original chunk.id
    // as the new turn's response_id (the agent's stable lineage id
    // is the same across turns). So new_response_id == spawn_response_id
    // is expected — no assertion that they differ.

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
