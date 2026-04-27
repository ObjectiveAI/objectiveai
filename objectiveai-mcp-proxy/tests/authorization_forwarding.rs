//! Upstream requires `Authorization: Bearer secret`. Without
//! `X-MCP-Authorization`, the proxy can't connect (upstream rejects);
//! the upstream is logged + dropped, and the client sees no tools from
//! it. With the right `X-MCP-Authorization`, the upstream connects
//! cleanly and tools appear.

mod common;

use std::collections::HashMap;

use common::{TestRig, UpstreamSpec};
use objectiveai_mcp_test_upstream::{TestTool, TestToolBehavior};

fn echo(name: &str) -> TestTool {
    TestTool { name: name.into(), description: None, behavior: TestToolBehavior::Echo }
}

#[tokio::test]
async fn missing_authorization_drops_the_upstream() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("private")
            .with_tools(vec![echo("hidden")])
            .with_require_auth("Bearer secret"),
    ])
    .await;

    let mut headers = HashMap::new();
    headers.insert("X-MCP-Servers", rig.x_mcp_servers());
    // Deliberately omit X-MCP-Authorization.
    let client = rig.connect_client(headers).await;

    let tools = client.peer().list_all_tools().await.expect("list_all_tools");
    assert!(
        tools.is_empty(),
        "upstream that rejected our auth should not contribute tools, got {:?}",
        tools.iter().map(|t| &t.name).collect::<Vec<_>>(),
    );

    client.cancel().await.ok();
}

#[tokio::test]
async fn correct_authorization_lets_the_upstream_in() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("private")
            .with_tools(vec![echo("hidden")])
            .with_require_auth("Bearer secret"),
    ])
    .await;

    let auth_map = serde_json::json!({
        rig.upstreams[0].url.clone(): "Bearer secret",
    });

    let mut headers = HashMap::new();
    headers.insert("X-MCP-Servers", rig.x_mcp_servers());
    headers.insert("X-MCP-Authorization", auth_map.to_string());
    let client = rig.connect_client(headers).await;

    let tools = client.peer().list_all_tools().await.expect("list_all_tools");
    let names: Vec<String> = tools.into_iter().map(|t| t.name.into()).collect();
    assert_eq!(names, vec!["private_hidden".to_string()]);

    client.cancel().await.ok();
}
