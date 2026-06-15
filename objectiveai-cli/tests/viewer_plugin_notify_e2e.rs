//! End-to-end test: a plugin's viewer extension receives a host
//! notification and acts on it.
//!
//! The CLI auto-spawns the real viewer, which mounts the committed
//! `testorg/viewer-notify` fixture plugin's iframe (kept alive even
//! when its tab isn't focused — see `objectiveai-viewer/src/App.tsx`).
//! A POST to the plugin's `/notify` route drives an `inbound` event
//! into that iframe, whose JS applies an agent tag via the viewer → cli
//! reverse channel (`agents tags apply`). We prove the notification was
//! received by enqueuing a message to that tag: `agents enqueue
//! --agent-tag <tag>` errors `TagNotFound` until the tag exists and
//! succeeds once the iframe has applied it.
//!
//! The test never touches the viewer lifecycle — the first `viewer
//! send` brings it up. Build the viewer frontend first so the
//! always-mounted-iframes change is in `dist/` (the spawned viewer
//! loads `frontendDist`, not Vite):
//!
//!   pnpm --filter objectiveai-viewer run build
//!   cargo test -p objectiveai-cli --test viewer_plugin_notify_e2e

mod cli_test_util;

use std::process::{Command, Output};
use std::time::{Duration, Instant};

#[tokio::test]
async fn viewer_plugin_route_applies_tag_verified_by_enqueue() {
    let _state_dir = cli_test_util::test_base_dir();
    let cli = cli_test_util::cli_binary();
    let dir = cli_test_util::objectiveai_dir();
    let state = cli_test_util::test_state_name();

    // Unique tag per run so a stale row (if cleanup didn't run) can't
    // make this pass spuriously. The fixture reads the name from the
    // notification body.
    let tag = format!("viewer-notify-{state}");
    let body = serde_json::json!({ "tag": tag }).to_string();

    let run = |args: &[&str]| -> Output {
        Command::new(&cli)
            .env("OBJECTIVEAI_DIR", &dir)
            .env("OBJECTIVEAI_STATE", &state)
            .args(args)
            .output()
            .expect("failed to run cli")
    };

    // Retry loop. The first `viewer send` auto-spawns the viewer; its
    // frontend then has to load, mount the plugin iframe, and have the
    // bridge subscribe — all async, and an `inbound` event emitted
    // before the subscription exists is dropped (Tauri emits aren't
    // per-listener buffered). So re-send each iteration (tag application
    // is idempotent) and poll by attempting the enqueue, which succeeds
    // only once the tag exists.
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut last_enqueue: Option<Output> = None;
    let applied = loop {
        let _ = run(&["viewer", "send", "/plugin/viewer-notify/notify", &body]);

        let enqueue = run(&["agents", "enqueue", "--agent-tag", &tag, "--simple", "ping"]);
        if enqueue.status.success() {
            break true;
        }
        last_enqueue = Some(enqueue);

        if Instant::now() >= deadline {
            break false;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    };

    assert!(
        applied,
        "tag `{tag}` was never applied within the deadline — the plugin \
         iframe did not receive the `/notify` notification or did not \
         apply the tag. last `agents enqueue` stderr: {}",
        last_enqueue
            .map(|o| String::from_utf8_lossy(&o.stderr).into_owned())
            .unwrap_or_default(),
    );
}
