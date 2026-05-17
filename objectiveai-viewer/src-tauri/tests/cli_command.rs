//! `cli_command` flow snapshot test.
//!
//! Drives `cli_run_impl` with a small deterministic command (no
//! upstream API needed) and snapshots the resulting
//! `Event::CliCommand` JSONL stream.

mod common;

use std::time::Duration;

use common::{ViewerTestEnv, is_cli_command_end, snapshot, test_api_address};

const SNAPSHOT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/snapshots/cli_command_schemas_list.jsonl");

#[tokio::test]
async fn cli_command_schemas_list() {
    if test_api_address().is_none() {
        eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping");
        return;
    }
    let mut env = ViewerTestEnv::new();

    // Pick an offline cli command that emits a small, deterministic
    // JSONL stream: `schemas viewer list` enumerates the JSON schema
    // titles exported from the viewer module — purely local, no
    // network or filesystem state.
    let args = vec![
        "objectiveai".to_string(),
        "schemas".to_string(),
        "viewer".to_string(),
        "list".to_string(),
    ];
    objectiveai_viewer::test_internals::cli_run_impl(
        env.events_tx.clone(),
        args,
        "test-iframe".to_string(),
    )
    .await
    .expect("cli_run_impl returned an error");

    let events = env
        .drain_until_end(is_cli_command_end, Duration::from_secs(30))
        .await;

    let actual = snapshot::events_to_jsonl(&events);
    snapshot::assert_snapshot(&actual, SNAPSHOT_PATH, include_str!("snapshots/cli_command_schemas_list.jsonl"));
}
