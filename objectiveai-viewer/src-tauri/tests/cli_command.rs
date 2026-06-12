//! `cli_command` flow snapshot test.
//!
//! Drives `cli_run_impl` with a small deterministic command (no
//! upstream API needed) and snapshots the resulting
//! `Event::CliCommand` JSONL stream — including the synthetic
//! `{"type":"end"}` terminator the forwarder appends.

mod common;

use std::time::Duration;

use common::{ViewerTestEnv, snapshot};
use objectiveai_sdk::cli::command::binary::BinaryExecutor;

const SNAPSHOT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/snapshots/cli_command_config_viewer_get.jsonl"
);

#[tokio::test]
async fn cli_command_config_viewer_get() {
    // The forwarder spawns the repo's committed cargo-run cli shim
    // out of the shared test root — no pre-build, no env plumbing.
    let dir = common::objectiveai_dir();
    let cli_binary = dir.join("bin").join(if cfg!(windows) {
        "objectiveai.exe"
    } else {
        "objectiveai"
    });
    let executor = BinaryExecutor::from_path(cli_binary)
        .env("OBJECTIVEAI_DIR", dir.to_string_lossy().into_owned())
        .env("OBJECTIVEAI_STATE", "viewer_cli_command");

    let env = ViewerTestEnv::new();

    // Pick an offline cli command that emits a small, deterministic
    // JSONL stream: `config viewer get` against the dedicated
    // `viewer_cli_command` state prints exactly one line — the empty
    // viewer config object — purely local, no network state.
    let args = vec![
        "objectiveai".to_string(),
        "config".to_string(),
        "viewer".to_string(),
        "get".to_string(),
    ];
    objectiveai_viewer::test_internals::cli_run_impl(
        &executor,
        env.events_tx.clone(),
        args,
        "test-iframe".to_string(),
    )
    .await
    .expect("cli_run_impl returned an error");

    let events = env.drain_until_close(Duration::from_secs(30)).await;

    let actual = snapshot::events_to_jsonl(&events);
    snapshot::assert_snapshot(
        &actual,
        SNAPSHOT_PATH,
        include_str!("snapshots/cli_command_config_viewer_get.jsonl"),
    );
}
