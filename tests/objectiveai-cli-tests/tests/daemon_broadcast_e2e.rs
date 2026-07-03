//! Wire-contract e2e for the daemon's `plugins/run` broadcast.
//!
//! The viewer's JS bridge routes daemon frames to plugin tabs keyed on
//! exactly this shape (see `objectiveai-viewer/src/plugin-bridge.ts`):
//!
//!   - request frame:  `{…context, id, value}` — NO top-level
//!     `path_type`; `value.path_type == "plugins/run"`, `value.name`
//!     is the plugin name.
//!   - response frame: `{id, path_type: "plugins/run", value}` with
//!     the same `id`.
//!
//! The bridge's own routing tests
//! (`objectiveai-viewer/src/plugin-bridge.test.ts`) use synthetic
//! frames; this test pins the real wire so those synthetics can't
//! silently drift from what the daemon actually broadcasts.

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
    Path as RunPath, Request as RunRequest, ResponseItem as RunItem,
};
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
async fn plugins_run_broadcast_frame_contract() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    // Ensure the daemon is up (idempotent — the run tee would spawn it
    // anyway) and read its published ws:// URL from the lock.
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
    assert!(url.starts_with("ws://"), "lock content should be a ws:// URL, got {url:?}");

    // Connect BEFORE producing, the way the viewer does, so no frame
    // of our run can be missed.
    let (mut ws, _response) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("WS connect to the daemon broadcast should succeed");

    // Produce: run the committed hello fixture. Its single notification
    // is the bare `{"hello":"world"}` payload.
    let run_items: Vec<RunItem> =
        cli_test_util::collect_stream(&executor, hello_run_request()).await;
    assert!(
        run_items.iter().any(|i| matches!(
            i,
            RunItem::Notification(v) if v.pointer("/hello").and_then(|h| h.as_str()) == Some("world")
        )),
        "the direct plugins run stream should carry the hello notification, got {run_items:?}",
    );

    // Consume the broadcast until we've seen our run's request frame
    // and its hello-notification response frame. The daemon carries
    // every teed run in this state (the `daemon spawn` command above,
    // daemon-launched plugins' nested commands, …) — frames with other
    // ids are expected noise and are ignored.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let mut run_id: Option<String> = None;
    let mut frames_for_run: Vec<serde_json::Value> = Vec::new();
    let mut saw_hello_response = false;
    while !saw_hello_response {
        let remaining = deadline
            .checked_duration_since(tokio::time::Instant::now())
            .expect("timed out waiting for the plugins/run frames on the daemon broadcast");
        let message = tokio::time::timeout(remaining, ws.next())
            .await
            .expect("timed out waiting for the next broadcast frame")
            .expect("the broadcast stream should not end before our frames arrive")
            .expect("the broadcast stream should not error");
        let Message::Text(text) = message else {
            continue;
        };
        let frame: serde_json::Value =
            serde_json::from_str(&text).expect("every broadcast frame should be JSON");

        match &run_id {
            None => {
                // Looking for OUR request frame: no top-level path_type,
                // value.path_type == "plugins/run", value.name == "hello".
                if frame.get("path_type").is_none()
                    && frame.pointer("/value/path_type").and_then(|p| p.as_str())
                        == Some("plugins/run")
                    && frame.pointer("/value/name").and_then(|n| n.as_str()) == Some("hello")
                {
                    let id = frame
                        .get("id")
                        .and_then(|i| i.as_str())
                        .expect("the request frame should carry a string id")
                        .to_string();
                    assert!(
                        frame.get("agent_instance_hierarchy").is_some(),
                        "the request frame should carry the producer context, got {frame}",
                    );
                    assert_eq!(
                        frame.pointer("/value/owner").and_then(|o| o.as_str()),
                        Some("objectiveai"),
                        "request frame value.owner",
                    );
                    run_id = Some(id);
                    frames_for_run.push(frame);
                }
            }
            Some(id) => {
                if frame.get("id").and_then(|i| i.as_str()) != Some(id.as_str()) {
                    continue;
                }
                // Every subsequent frame for our id is a response frame:
                // top-level path_type mirrors the request's path.
                assert_eq!(
                    frame.get("path_type").and_then(|p| p.as_str()),
                    Some("plugins/run"),
                    "response frame path_type, got {frame}",
                );
                saw_hello_response |= frame.pointer("/value/hello").and_then(|h| h.as_str())
                    == Some("world");
                frames_for_run.push(frame);
            }
        }
    }
    kill_daemon(&executor).await;

    // Exactly one request frame for our id, and it came first.
    let request_frames = frames_for_run
        .iter()
        .filter(|f| f.get("path_type").is_none())
        .count();
    assert_eq!(
        request_frames, 1,
        "exactly one request frame for the run, got {frames_for_run:?}",
    );
    assert!(
        frames_for_run[0].get("path_type").is_none(),
        "the request frame should precede its responses, got {frames_for_run:?}",
    );
}
