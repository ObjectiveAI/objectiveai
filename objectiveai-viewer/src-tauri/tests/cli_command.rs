//! `cli_command` flow snapshot test.
//!
//! Drives `cli_run_impl` with a small deterministic command (no
//! upstream API needed) and snapshots the resulting
//! `Event::CliCommand` JSONL stream — including the synthetic
//! `{"type":"end"}` terminator the forwarder appends.

mod common;

use std::time::Duration;

use common::{ViewerTestEnv, snapshot, test_api_address};
use objectiveai_sdk::cli::command::binary::BinaryExecutor;

const SNAPSHOT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/snapshots/cli_command_config_viewer_get.jsonl"
);

#[tokio::test]
async fn cli_command_config_viewer_get() {
    if test_api_address().is_none() {
        eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping");
        return;
    }
    // The forwarder spawns a real cli binary now; test.sh builds one
    // and exports its path. Skip-gate mirrors OBJECTIVEAI_TEST_PORT
    // so a bare `cargo test -p objectiveai-viewer` stays green.
    let Some(cli_binary) = std::env::var_os("OBJECTIVEAI_CLI_BINARY") else {
        eprintln!("OBJECTIVEAI_CLI_BINARY not set — skipping");
        return;
    };
    let scratch = std::env::temp_dir().join(format!(
        "oai-viewer-cli-command-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&scratch).expect("create scratch OBJECTIVEAI_DIR");
    let executor = BinaryExecutor::from_path(cli_binary)
        .env("OBJECTIVEAI_DIR", scratch.to_string_lossy().into_owned());

    let env = ViewerTestEnv::new();

    // Pick an offline cli command that emits a small, deterministic
    // JSONL stream: `config viewer get` against the fresh scratch
    // OBJECTIVEAI_DIR prints exactly one line — the empty viewer
    // config object — purely local, no network state.
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
