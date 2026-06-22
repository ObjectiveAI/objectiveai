//! End-to-end tests for the plugin daemon.
//!
//! 1. `daemon_spawn_concurrent` — 10 `daemon spawn`s in true parallel all
//!    succeed (singleton lock + init gate).
//! 2. `daemon_runs_command_applies_tag` — the daemon launches the
//!    `daemon-echo` fixture (`daemon: true`) under the SHARED plugin
//!    executor; that plugin applies a tag to a mock agent via a nested
//!    command and records the tag in its echo file. We verify both the
//!    file content AND that the tag now resolves via `agents tags lookup`.

mod cli_test_util;

use std::time::Duration;

use futures::StreamExt;
use objectiveai_sdk::cli::command::CommandExecutor;
use objectiveai_sdk::cli::command::agents::tags::lookup::{
    Path as LookupPath, Request as LookupRequest, Response as LookupResponse,
};
use objectiveai_sdk::cli::command::daemon::kill::{
    Path as KillPath, Request as KillRequest, Response as KillResponse,
};
use objectiveai_sdk::cli::command::daemon::spawn::{
    Path as SpawnPath, Request as SpawnRequest, ResponseItem as SpawnItem,
};

const APPLIED_TAG: &str = "daemon-applied-tag";

fn spawn_request() -> SpawnRequest {
    SpawnRequest {
        path_type: SpawnPath::DaemonSpawn,
        dangerous_advanced: None,
        base: Default::default(),
    }
}

/// Best-effort daemon teardown so the detached daemon doesn't linger
/// across runs.
async fn kill_daemon<E: CommandExecutor>(executor: &E) {
    let request = KillRequest {
        path_type: KillPath::DaemonKill,
        base: Default::default(),
    };
    let _ = executor
        .execute_one::<KillRequest, KillResponse>(request, None)
        .await;
}

/// `<dir>/state/<state>/plugins/objectiveai/<name>/0.0.1/input.json` —
/// where the `daemon-echo` fixture records the tag it applied.
fn echo_input_path(name: &str) -> std::path::PathBuf {
    cli_test_util::objectiveai_dir()
        .join("state")
        .join(cli_test_util::test_state_name())
        .join("plugins")
        .join("objectiveai")
        .join(name)
        .join("0.0.1")
        .join("input.json")
}

#[tokio::test(flavor = "multi_thread")]
async fn daemon_spawn_concurrent() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;
    let executor_ref = &executor;

    let results = futures::future::join_all((0..10).map(|_| async move {
        let stream = executor_ref
            .execute::<SpawnRequest, SpawnItem>(spawn_request(), None)
            .await
            .map_err(|e| format!("execute failed: {e:?}"))?;
        let mut stream = std::pin::pin!(stream);
        let mut ok = false;
        while let Some(item) = stream.next().await {
            match item {
                Ok(i) => ok |= i.ok,
                Err(e) => return Err(format!("stream error: {e:?}")),
            }
        }
        if ok {
            Ok(())
        } else {
            Err("no ok item".to_string())
        }
    }))
    .await;

    let failures: Vec<String> = results.into_iter().filter_map(Result::err).collect();
    kill_daemon(&executor).await;
    assert!(
        failures.is_empty(),
        "{} of 10 concurrent daemon spawns failed: {failures:?}",
        failures.len(),
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn daemon_runs_command_applies_tag() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    // Spawn the daemon — it launches daemon-echo under the shared plugin
    // executor; daemon-echo applies a tag via a nested command, then
    // records the tag name in its echo file.
    let spawn_items: Vec<SpawnItem> = cli_test_util::collect_stream(&executor, spawn_request()).await;
    assert!(spawn_items.iter().any(|i| i.ok), "daemon should spawn");

    // The echo file appears only after the daemon spawns daemon-echo and its
    // nested postgres-backed apply completes. Under the full integration suite
    // (many tests each running their own postgres cluster in parallel) that can
    // take well over 10s, so poll generously (30s) to tolerate contention.
    let path = echo_input_path("daemon-echo");
    let mut recorded: Option<String> = None;
    for _ in 0..300 {
        if let Ok(contents) = std::fs::read_to_string(&path)
            && let Ok(serde_json::Value::String(tag)) =
                serde_json::from_str::<serde_json::Value>(&contents)
        {
            recorded = Some(tag);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let recorded = recorded.expect("daemon-echo should record the applied tag in its echo file");
    assert_eq!(recorded, APPLIED_TAG, "echo file content");

    // Verify the daemon's nested command actually applied the tag: a tag
    // lookup must resolve it (not Absent).
    let lookup = LookupRequest::Tag {
        path_type: LookupPath::AgentsTagsLookup,
        tag: recorded.clone(),
        base: Default::default(),
    };
    let response: LookupResponse = cli_test_util::execute_one(&executor, lookup).await;
    let resolved = !matches!(response, LookupResponse::Absent);
    kill_daemon(&executor).await;
    assert!(
        resolved,
        "tag lookup for {recorded:?} should resolve, got {response:?}",
    );
}
