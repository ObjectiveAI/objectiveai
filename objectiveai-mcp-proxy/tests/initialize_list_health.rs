//! The proxy's `initialize` succeeds only if every upstream also passes a
//! post-connect health probe: `tools/list` AND `resources/list` must both
//! succeed. A handshake that succeeds against an upstream which then errors
//! on either list must fail the whole `initialize` — same outcome as a
//! connect failure.
//!
//! NOTE on timing: a `tools/list` / `resources/list` error is currently
//! retried as transient by the SDK mcp client until the proxy's
//! `mcp_backoff_max_elapsed_time` (default 40s), so the two failure tests
//! take roughly that long to surface the error before asserting. They are
//! correct, just slow.

mod common;

use std::collections::HashMap;

use common::{TestRig, UpstreamSpec};
use test_upstream::{TestResource, TestTool, TestToolBehavior};

fn servers_header(rig: &TestRig) -> HashMap<&'static str, String> {
    let mut headers = HashMap::new();
    headers.insert("X-MCP-Servers", rig.x_mcp_servers());
    headers
}

/// Both lists resolve → the probe passes → `initialize` completes.
#[tokio::test]
async fn initialize_succeeds_when_both_lists_ok() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("upstream-a")
            .with_tools(vec![TestTool {
                name: "echo".into(),
                description: None,
                behavior: TestToolBehavior::Static { reply: "ok".into() },
            }])
            .with_resources(vec![TestResource {
                uri: "mem://r1".into(),
                name: Some("r1".into()),
                text: "hello".into(),
            }]),
    ])
    .await;

    let client = rig.connect_client(servers_header(&rig)).await;
    assert!(
        client.peer_info().is_some(),
        "initialize should have completed when both lists succeed",
    );
    client.cancel().await.ok();
}

/// `tools/list` errors (initialize + resources/list still succeed) → the
/// probe fails → `initialize` fails.
#[tokio::test]
async fn initialize_fails_when_tools_list_fails() {
    let rig = TestRig::new(vec![UpstreamSpec::new("upstream-a")]).await;
    rig.set_upstream_list_failure(0, true, false).await;

    let result = rig.try_connect_client(servers_header(&rig)).await;
    assert!(
        result.is_err(),
        "initialize must fail when an upstream's tools/list errors, got Ok",
    );
}

/// `resources/list` errors (initialize + tools/list still succeed) → the
/// probe fails → `initialize` fails.
#[tokio::test]
async fn initialize_fails_when_resources_list_fails() {
    let rig = TestRig::new(vec![UpstreamSpec::new("upstream-a")]).await;
    rig.set_upstream_list_failure(0, false, true).await;

    let result = rig.try_connect_client(servers_header(&rig)).await;
    assert!(
        result.is_err(),
        "initialize must fail when an upstream's resources/list errors, got Ok",
    );
}
