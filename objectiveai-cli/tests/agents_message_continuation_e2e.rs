//! `agents spawn` → wait for cli-stream to finish →
//! `agents message` → assert the second turn's request body's
//! `continuation` field byte-equals the first turn's continuation
//! (the one upserted into `agent_continuations` by the spawn's
//! final chunk).
//!
//! The test is the smoking gun for response-side continuation
//! propagation. Reverting the SDK fix that taught `lookup_session`
//! to read continuation from the `agent_continuations` registry
//! (vs parsing the cumulative response blob) would surface as a
//! mismatch between the two values pulled below.
//!
//! Driven through the SDK `BinaryExecutor` for every cli leaf
//! invocation, including the postgres reads — `db query` reads
//! `agent_continuations` directly and `logs.agent_completion_requests`'s
//! `body->>continuation` for the new turn.

mod cli_test_util;

use std::time::Duration;

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::message::{
    MessageTarget, Request as MessageRequest,
    RequestDangerousAdvanced as MessageDangerousAdvanced, RequestMessage,
    ResponseItem as MessageResponseItem,
};
use objectiveai_sdk::cli::command::agents::spawn::{
    AgentResolution, AgentSpec, Request as SpawnRequest, RequestDangerousAdvanced,
    ResponseItem as SpawnResponseItem,
};

#[tokio::test]
async fn spawn_then_message_propagates_response_continuation() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!(
            "OBJECTIVEAI_TEST_PORT not set — skipping spawn_then_message_propagates_response_continuation"
        );
        return;
    }

    let base_dir = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    // ── 1. Spawn a mock agent ────────────────────────────────────
    // `dangerous_advanced.stream = true` keeps the parent cli
    // attached to its instance subprocess and forwards every
    // `AgentCompletionChunk` as `SpawnResponseItem::Chunk(_)`.
    // `collect_stream` returning then implies the runner exited,
    // the LogWriter flushed, and `agent_continuations` carries
    // the first turn's continuation.
    let spawn_request = SpawnRequest {
        path_type: objectiveai_sdk::cli::command::agents::spawn::Path::AgentsSpawn,
        message: RequestMessage::Simple("first turn".to_string()),
        agent: AgentResolution::Direct {
            agent_spec: AgentSpec::Resolved(
                serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
                    serde_json::json!({"upstream":"mock","output_mode":"instruction"}),
                )
                .expect("inline mock agent must deserialize"),
            ),
        },
        dangerous_advanced: Some(RequestDangerousAdvanced {
            stream: Some(true),
            seed: Some(42),
        }),
        jq: None,
    };
    let spawn_items: Vec<SpawnResponseItem> =
        cli_test_util::collect_stream(&executor, spawn_request).await;
    // `chunk.agent_instance_hierarchy` is the full lineage the cli
    // writes to `logs.messages` and `agent_continuations` (the api
    // server's slot id, shape `cli/{agent_full_id}-{response_id}`).
    // It's the only string that lines up with what's in the DB.
    let spawn_chunk_aih = spawn_items
        .iter()
        .find_map(|item| match item {
            SpawnResponseItem::Chunk(chunk) if !chunk.agent_instance_hierarchy.is_empty() => {
                Some(chunk.agent_instance_hierarchy.clone())
            }
            _ => None,
        })
        .expect("agents spawn must emit a Chunk with a non-empty agent_instance_hierarchy");
    let spawn_response_id = spawn_items
        .iter()
        .find_map(|item| match item {
            SpawnResponseItem::Chunk(chunk) if !chunk.id.is_empty() => Some(chunk.id.clone()),
            _ => None,
        })
        .expect("agents spawn must emit a Chunk with non-empty id");

    // ── 2. Capture the spawn's continuation from postgres ───────
    // `agent_continuations` is upserted per-chunk; by the time
    // collect_stream returns, the row holds the final continuation
    // value for the chunk's full AIH.
    let spawn_continuation =
        cli_test_util::wait_for_continuation(&executor, &spawn_chunk_aih, Duration::from_secs(30))
            .await;

    // ── 3. Message the agent (forces a new turn) ────────────────
    // Split `{parent}/{instance}` so the message handler composes
    // the same full AIH the spawn turn registered.
    let (parent, instance) = spawn_chunk_aih
        .rsplit_once('/')
        .map(|(p, i)| (Some(p.to_string()), i.to_string()))
        .expect("spawn_chunk_aih must carry at least one '/'");
    let message_request = MessageRequest {
        path_type: objectiveai_sdk::cli::command::agents::message::Path::AgentsMessage,
        target: MessageTarget::Direct {
            parent_agent_instance_hierarchy: parent,
            agent_instance: instance,
        },
        message: RequestMessage::Simple("follow up".to_string()),
        enqueue: None,
        dangerous_advanced: Some(MessageDangerousAdvanced {
            stream: Some(true),
            seed: Some(42),
        }),
        jq: None,
    };
    let items: Vec<MessageResponseItem> =
        cli_test_util::collect_stream(&executor, message_request).await;
    let new_response_id = items
        .iter()
        .find_map(|item| match item {
            MessageResponseItem::Chunk(chunk) if !chunk.id.is_empty() => Some(chunk.id.clone()),
            _ => None,
        })
        .unwrap_or(spawn_response_id);

    // ── 4. Read the new turn's request body's continuation ──────
    // `logs.agent_completion_requests.body->>'continuation'` is
    // exactly what the cli stamped onto the second turn's request
    // before sending it upstream.
    let request_continuation =
        cli_test_util::wait_for_request_continuation(&executor, &new_response_id, Duration::from_secs(30))
            .await
            .expect("second turn's request body must carry a continuation");

    // ── 5. The smoking gun ──────────────────────────────────────
    assert_eq!(
        request_continuation, spawn_continuation,
        "second turn's request continuation must equal first turn's agent_continuations value",
    );
}
