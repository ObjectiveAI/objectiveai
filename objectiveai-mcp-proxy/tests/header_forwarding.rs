//! Upstream's `header_gate = ("X-Trace-Id", "abc")` requires that
//! exact header on every MCP request. With the right
//! `X-MCP-Headers: {"X-Trace-Id":"abc"}` everything works; without it
//! (or with the wrong value) the upstream drops out.

mod common;

use std::collections::HashMap;

use common::{TestRig, UpstreamSpec};
use objectiveai_mcp_test_upstream::{TestTool, TestToolBehavior};

fn echo(name: &str) -> TestTool {
    TestTool { name: name.into(), description: None, behavior: TestToolBehavior::Echo }
}

#[tokio::test]
async fn correct_header_lets_upstream_through() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("gated")
            .with_tools(vec![echo("ok")])
            .with_header_gate("X-Trace-Id", "abc"),
    ])
    .await;

    let mcp_headers = serde_json::json!({ "X-Trace-Id": "abc" });
    let mut headers = HashMap::new();
    headers.insert("X-MCP-Servers", rig.x_mcp_servers());
    headers.insert("X-MCP-Headers", mcp_headers.to_string());
    let client = rig.connect_client(headers).await;

    let names: Vec<String> = client
        .peer()
        .list_all_tools()
        .await
        .expect("list_all_tools")
        .into_iter()
        .map(|t| t.name.into())
        .collect();
    assert_eq!(names, vec!["gated_ok".to_string()]);

    let seen = rig.upstream_seen_headers(0).await;
    assert_eq!(
        seen.get("x-trace-id").map(String::as_str),
        Some("abc"),
        "upstream should have seen the forwarded X-Trace-Id; got {seen:?}",
    );

    client.cancel().await.ok();
}

#[tokio::test]
async fn wrong_header_drops_upstream() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("gated")
            .with_tools(vec![echo("ok")])
            .with_header_gate("X-Trace-Id", "abc"),
    ])
    .await;

    let mcp_headers = serde_json::json!({ "X-Trace-Id": "wrong" });
    let mut headers = HashMap::new();
    headers.insert("X-MCP-Servers", rig.x_mcp_servers());
    headers.insert("X-MCP-Headers", mcp_headers.to_string());
    let client = rig.connect_client(headers).await;

    let tools = client.peer().list_all_tools().await.expect("list_all_tools");
    assert!(tools.is_empty(), "wrong header should drop the upstream");

    client.cancel().await.ok();
}
