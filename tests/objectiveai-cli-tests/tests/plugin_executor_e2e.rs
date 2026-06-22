//! End-to-end tests for the SDK in-process plugin executor, exercised against
//! the real CLI. A plugin written in JavaScript, one in Python, and one in Go
//! each use their language's `PluginCommandExecutor` + the generated
//! `agents tags apply` execute fn to apply a tag to a mock agent over the NDJSON
//! command protocol. We run each via `plugins run` and assert (a) the plugin
//! emitted its notification (so it actually ran through the executor) and (b)
//! the tag now resolves via `agents tags lookup` (so the mutation is real).
//!
//! The tests are identical except for the plugin coordinate + the tag it
//! applies. They are the only end-to-end coverage of the JS/Python/Go plugin
//! executors against a live host.

mod cli_test_util;

use objectiveai_sdk::cli::command::agents::tags::lookup::{
    Path as LookupPath, Request as LookupRequest, Response as LookupResponse,
};
use objectiveai_sdk::cli::command::plugins::run::{
    Path as RunPath, Request as RunRequest, ResponseItem as RunItem,
};

fn run_request(name: &str) -> RunRequest {
    RunRequest {
        path_type: RunPath::PluginsRun,
        owner: "objectiveai".to_string(),
        name: name.to_string(),
        version: "0.0.1".to_string(),
        args: Vec::new(),
        base: Default::default(),
    }
}

/// Run `plugin` via `plugins run`, assert it emitted a notification carrying
/// `expected_tag`, then assert the tag it applied resolves (not `Absent`).
async fn assert_plugin_applies_tag(plugin: &str, expected_tag: &str) {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    // `plugins run` blocks until the plugin exits — which happens only after
    // the nested `agents tags apply` has committed — so no polling is needed.
    let items: Vec<RunItem> = cli_test_util::collect_stream(&executor, run_request(plugin)).await;

    // The plugin emits one notification carrying the tag it applied (proving it
    // drove the executor to completion).
    let saw_notification = items.iter().any(|item| match item {
        RunItem::Notification(value) => {
            value.get("applied").and_then(|a| a.as_str()) == Some(expected_tag)
        }
        _ => false,
    });
    assert!(
        saw_notification,
        "{plugin}: expected a notification with applied={expected_tag:?}, got {items:?}",
    );

    // The mutation must be real: a tag lookup resolves it (not Absent).
    let lookup = LookupRequest::Tag {
        path_type: LookupPath::AgentsTagsLookup,
        tag: expected_tag.to_string(),
        base: Default::default(),
    };
    let response: LookupResponse = cli_test_util::execute_one(&executor, lookup).await;
    assert!(
        !matches!(response, LookupResponse::Absent),
        "{plugin}: tag {expected_tag:?} should resolve, got {response:?}",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn plugin_executor_js_applies_tag() {
    assert_plugin_applies_tag("tags-apply-js", "js-plugin-applied-tag").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn plugin_executor_py_applies_tag() {
    assert_plugin_applies_tag("tags-apply-py", "py-plugin-applied-tag").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn plugin_executor_go_applies_tag() {
    assert_plugin_applies_tag("tags-apply-go", "go-plugin-applied-tag").await;
}
