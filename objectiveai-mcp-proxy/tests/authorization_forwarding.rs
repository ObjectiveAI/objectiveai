//! Upstream requires `Authorization: Bearer secret`. Without
//! `X-MCP-Authorization`, the proxy fails the entire `initialize` with
//! a JSON-RPC `-32603` (`connect_all` fans out via `try_join_all` and
//! surfaces the first failure). With the right `X-MCP-Authorization`,
//! the upstream connects cleanly and its tools appear.

mod common;

use std::collections::HashMap;

use common::{TestRig, UpstreamSpec};
use rmcp::ServiceExt;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use test_upstream::{TestTool, TestToolBehavior};

fn echo(name: &str) -> TestTool {
    TestTool { name: name.into(), description: None, behavior: TestToolBehavior::Echo }
}

#[tokio::test]
async fn missing_authorization_fails_initialize() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("private")
            .with_tools(vec![echo("hidden")])
            .with_require_auth("Bearer secret"),
    ])
    .await;

    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-mcp-servers"),
        HeaderValue::from_str(&rig.x_mcp_servers()).unwrap(),
    );
    // Deliberately omit X-MCP-Authorization.

    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(rig.proxy.url.clone())
            .custom_headers(headers.into_iter().filter_map(|(n, v)| Some((n?, v))).collect()),
    );

    // initialize is what fans out per-upstream connects; it should fail.
    let result = client_info_for_test().serve(transport).await;
    let err = result.err().expect("initialize should fail with -32603");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("-32603") && msg.contains("upstream connect failed"),
        "unexpected error: {msg}",
    );
}

fn client_info_for_test() -> rmcp::model::ClientInfo {
    let value = serde_json::json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {},
        "clientInfo": { "name": "auth-test", "version": "0.1.0" },
    });
    serde_json::from_value(value).expect("ClientInfo deserialize")
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
