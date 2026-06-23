//! E2E: `--no-objectiveai` disables the embedded python's
//! `objectiveai.execute(...)` host call — it raises instead of
//! dispatching a CLI command.
//!
//! Covered both ways:
//! - a DIRECT `python` command run with `no_objectiveai = true`, and
//! - a PER-STREAM-ITEM `--python` output transform (over a mock agent's
//!   streamed messages), where the cli auto-applies `--no-objectiveai`.
//!
//! In both cases the guest raises, which surfaces as a `PythonException`
//! the cli re-emits as an error line — parsed by the SDK executor into a
//! stream `Err`. So we drive the RAW stream (not `execute_one` /
//! `collect_stream`, which `.expect()` success) and assert an `Err`
//! whose message mentions `--no-objectiveai`.

mod cli_test_util;

use futures::StreamExt;
use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::message::RequestMessage;
use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
use objectiveai_sdk::cli::command::agents::spawn::{
    Path as SpawnPath, Request as SpawnRequest, RequestDangerousAdvanced,
};
use objectiveai_sdk::cli::command::python::{Path as PyPath, Request as PyRequest, Response};
use objectiveai_sdk::cli::command::{CommandExecutor, CommandRequest, CommandResponse, RequestBase};

/// A python snippet that calls the gated host function.
const CALL: &str = r#"objectiveai.execute(["agents", "tags", "lookup", "--tag", "x"])"#;

/// Drive the raw stream and assert an `Err` mentioning `--no-objectiveai`
/// surfaced (the disabled `objectiveai.execute` raising in the guest).
async fn assert_disabled<R, T>(
    executor: &cli_test_util::HangPreventingBinaryCommandExecutor,
    request: R,
) where
    R: CommandRequest + Send + serde::Serialize,
    T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
{
    let stream = executor
        .execute::<R, T>(request, None)
        .await
        .expect("cli execute must start");
    let mut stream = std::pin::pin!(stream);
    let mut saw_disabled = false;
    while let Some(item) = stream.next().await {
        if let Err(e) = item {
            if format!("{e:?}").contains("--no-objectiveai") {
                saw_disabled = true;
            }
        }
    }
    assert!(
        saw_disabled,
        "expected objectiveai.execute to be disabled (--no-objectiveai)",
    );
}

/// Direct `python --no-objectiveai`: `objectiveai.execute` raises.
#[tokio::test(flavor = "multi_thread")]
async fn no_objectiveai_blocks_direct_execute() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    let request = PyRequest {
        path_type: PyPath::Python,
        code: CALL.to_string(),
        input: None,
        no_objectiveai: Some(true),
        base: Default::default(),
    };
    assert_disabled::<PyRequest, Response>(&executor, request).await;
}

/// Per-stream-item `--python` transform: the cli auto-disables
/// `objectiveai.execute`. Spawn a mock agent and transform each streamed
/// item with a snippet that calls the (now disabled) host function.
#[tokio::test(flavor = "multi_thread")]
async fn no_objectiveai_auto_for_python_transform() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    let spec = serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
        serde_json::json!({
            "upstream": "mock",
            "output_mode": "instruction",
            "instruction": "hi"
        }),
    )
    .expect("mock agent spec deserializes");

    let request = SpawnRequest {
        path_type: SpawnPath::AgentsSpawn,
        message: RequestMessage::Simple("hi".to_string()),
        agent: AgentSelector::Ref {
            agent: AgentRef::Resolved(spec),
        },
        dangerous_advanced: Some(RequestDangerousAdvanced {
            stream: Some(true),
            seed: Some(1),
            skip_lock: None,
        }),
        // A per-item python transform — the cli auto-applies
        // `--no-objectiveai` to it, so this `objectiveai.execute` raises.
        base: RequestBase {
            python: Some(CALL.to_string()),
            ..Default::default()
        },
    };
    // With a transform set, the cli emits transformed JSON values; we only
    // care about the error, so decode items as `serde_json::Value`.
    assert_disabled::<SpawnRequest, serde_json::Value>(&executor, request).await;
}
