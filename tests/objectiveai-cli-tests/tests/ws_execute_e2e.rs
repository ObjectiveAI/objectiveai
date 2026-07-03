//! End-to-end test for the daemon's `/execute` WebSocket route driven
//! through the SDK's [`WebSocketExecutor`] — both sides of the wire
//! contract in one test.
//!
//! Flow: spawn the daemon, read its published ws:// URL from the
//! `plugins-daemon` lock, point a `WebSocketExecutor` at `/execute`,
//! and run the committed hello fixture with an `AgentArguments`
//! identity override (`agent_instance_hierarchy: "Viewer"` — the same
//! override the real viewer sends). Assert:
//!
//! 1. the typed notification comes back through the executor stream
//!    (the daemon ran the command in-process and streamed the items);
//! 2. a `/listen` client sees the run's broadcast request frame with
//!    the OVERRIDDEN identity in its context — proving the per-request
//!    config override applied and the in-process run teed like any
//!    other CLI activity.

mod cli_test_util;

use std::time::Duration;

use futures::StreamExt;
use objectiveai_sdk::cli::command::CommandExecutor;
use objectiveai_sdk::cli::command::daemon::kill::{
    Path as KillPath, Request as KillRequest, Response as KillResponse,
};
use objectiveai_sdk::cli::command::daemon::spawn::{
    Path as SpawnPath, Request as SpawnRequest, ResponseItem as SpawnItem,
};
use objectiveai_sdk::cli::command::plugins::run::{
    Path as RunPath, Request as RunRequest,
};
use objectiveai_sdk::cli::command::websocket::WebSocketExecutor;
use objectiveai_sdk::cli::command::AgentArguments;
use tokio_tungstenite::tungstenite::Message;

fn spawn_request() -> SpawnRequest {
    SpawnRequest {
        path_type: SpawnPath::DaemonSpawn,
        dangerous_advanced: None,
        base: Default::default(),
    }
}

fn hello_run_request() -> RunRequest {
    RunRequest {
        path_type: RunPath::PluginsRun,
        owner: "objectiveai".to_string(),
        name: "hello".to_string(),
        version: "0.0.1".to_string(),
        args: vec!["world".to_string()],
        base: Default::default(),
    }
}

/// Best-effort daemon teardown so the detached daemon doesn't linger.
async fn kill_daemon<E: CommandExecutor>(executor: &E) {
    let request = KillRequest {
        path_type: KillPath::DaemonKill,
        base: Default::default(),
    };
    let _ = executor
        .execute_one::<KillRequest, KillResponse>(request, None)
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn ws_execute_runs_in_process_with_identity_override() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    // Ensure the daemon and read its published base ws:// URL.
    let spawn_items: Vec<SpawnItem> =
        cli_test_util::collect_stream(&executor, spawn_request()).await;
    assert!(spawn_items.iter().any(|i| i.ok), "daemon should spawn");
    let lock_dir = cli_test_util::objectiveai_dir()
        .join("state")
        .join(cli_test_util::test_state_name())
        .join("locks");
    let url = objectiveai_sdk::lockfile::try_read(&lock_dir, "plugins-daemon")
        .await
        .expect("reading the plugins-daemon lock should not error")
        .expect("the daemon lock should hold the ws:// URL");

    // A broadcast observer, connected BEFORE the execute so the run's
    // teed frames can't be missed.
    let (mut listen_ws, _response) =
        tokio_tungstenite::connect_async(format!("{url}/listen"))
            .await
            .expect("WS connect to /listen should succeed");

    // The executor under test, aimed at /execute — the viewer's exact
    // configuration (sans signature; no DAEMON_SECRET in tests).
    let ws_executor = WebSocketExecutor::new(format!("{url}/execute"));
    let agent_arguments = AgentArguments {
        agent_instance_hierarchy: Some("Viewer".to_string()),
        ..AgentArguments::default()
    };
    let mut stream = ws_executor
        .execute::<RunRequest, serde_json::Value>(hello_run_request(), Some(&agent_arguments))
        .await
        .expect("execute over /execute should connect and send");

    // 1. The typed items stream back over the execute connection; the
    // hello fixture emits its `{"hello":"world"}` notification.
    let mut saw_hello = false;
    while let Some(item) = stream.next().await {
        let item = item.expect("the execute stream should carry no transport/cli errors");
        saw_hello |= item.pointer("/hello").and_then(|h| h.as_str()) == Some("world");
    }
    assert!(saw_hello, "the execute stream should carry the hello notification");

    // 2. The run was teed: /listen carries its request frame, whose
    // context is the OVERRIDDEN identity, not the daemon's own.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let request_frame = loop {
        let remaining = deadline
            .checked_duration_since(tokio::time::Instant::now())
            .expect("timed out waiting for the run's broadcast request frame");
        let message = tokio::time::timeout(remaining, listen_ws.next())
            .await
            .expect("timed out waiting for the next broadcast frame")
            .expect("the broadcast stream should not end early")
            .expect("the broadcast stream should not error");
        let Message::Text(text) = message else {
            continue;
        };
        let frame: serde_json::Value =
            serde_json::from_str(&text).expect("every broadcast frame should be JSON");
        // Our run's request frame: no top-level path_type, and the
        // request inside is the hello plugins/run.
        if frame.get("path_type").is_none()
            && frame.pointer("/value/path_type").and_then(|p| p.as_str()) == Some("plugins/run")
            && frame.pointer("/value/name").and_then(|n| n.as_str()) == Some("hello")
        {
            break frame;
        }
    };
    kill_daemon(&executor).await;
    assert_eq!(
        request_frame
            .get("agent_instance_hierarchy")
            .and_then(|h| h.as_str()),
        Some("Viewer"),
        "the broadcast request frame should carry the overridden identity, got {request_frame}",
    );
}
