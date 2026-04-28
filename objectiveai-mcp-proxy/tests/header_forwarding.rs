//! Upstream's `header_gate = ("X-Trace-Id", "abc")` requires that
//! exact header on every MCP request. With the right
//! `X-MCP-Headers: {"X-Trace-Id":"abc"}` everything works; with the
//! wrong value the proxy fails the entire `initialize` with a JSON-RPC
//! `-32603` (`connect_all` fans out via `try_join_all` and surfaces the
//! first failure).

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
async fn wrong_header_fails_initialize() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("gated")
            .with_tools(vec![echo("ok")])
            .with_header_gate("X-Trace-Id", "abc"),
    ])
    .await;

    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-mcp-servers"),
        HeaderValue::from_str(&rig.x_mcp_servers()).unwrap(),
    );
    headers.insert(
        HeaderName::from_static("x-mcp-headers"),
        HeaderValue::from_str(&serde_json::json!({ "X-Trace-Id": "wrong" }).to_string()).unwrap(),
    );

    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(rig.proxy.url.clone())
            .custom_headers(headers.into_iter().filter_map(|(n, v)| Some((n?, v))).collect()),
    );

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
        "clientInfo": { "name": "header-test", "version": "0.1.0" },
    });
    serde_json::from_value(value).expect("ClientInfo deserialize")
}
