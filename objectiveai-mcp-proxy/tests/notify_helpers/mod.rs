//! Raw-HTTP helpers shared by the `/notify` integration tests. Each
//! notify test file declares `mod notify_helpers;` to pull these in.
//! We bypass rmcp here because we want explicit control over the
//! `Mcp-Session-Id` and the exact wire payloads of `tools/call`.

#![allow(dead_code)]

use serde_json::{Value, json};

pub const ACCEPT: &str = "application/json, text/event-stream";

/// Drive a fresh proxy session through `initialize` +
/// `notifications/initialized` and return the assigned `Mcp-Session-Id`.
pub async fn init_session(
    client: &reqwest::Client,
    proxy_url: &str,
    x_mcp_servers: &str,
) -> String {
    let resp = client
        .post(proxy_url)
        .header("Content-Type", "application/json")
        .header("Accept", ACCEPT)
        .header("X-MCP-Servers", x_mcp_servers)
        .body(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": { "name": "notify-test", "version": "0.1.0" },
                },
            })
            .to_string(),
        )
        .send()
        .await
        .expect("initialize");
    assert!(resp.status().is_success(), "initialize: {}", resp.status());
    let session_id = resp
        .headers()
        .get("Mcp-Session-Id")
        .expect("Mcp-Session-Id header present")
        .to_str()
        .unwrap()
        .to_string();
    let _ = parse_jsonrpc_body(resp).await;

    client
        .post(proxy_url)
        .header("Content-Type", "application/json")
        .header("Accept", ACCEPT)
        .header("Mcp-Session-Id", &session_id)
        .body(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
        .send()
        .await
        .expect("initialized notification");

    session_id
}

/// `POST /notify` with the given JSON body (a `Vec<ContentBlock>` shape).
pub async fn post_notify(
    client: &reqwest::Client,
    proxy_url: &str,
    session_id: &str,
    body: Value,
) -> reqwest::Response {
    let notify_url = format!("{proxy_url}notify");
    client
        .post(&notify_url)
        .header("Content-Type", "application/json")
        .header("Mcp-Session-Id", session_id)
        .body(body.to_string())
        .send()
        .await
        .expect("post /notify")
}

/// Issue a `tools/call` against the proxy and return the parsed JSON-RPC
/// response value.
pub async fn call_tool(
    client: &reqwest::Client,
    proxy_url: &str,
    session_id: &str,
    request_id: u64,
    tool_name: &str,
) -> Value {
    let resp = client
        .post(proxy_url)
        .header("Content-Type", "application/json")
        .header("Accept", ACCEPT)
        .header("Mcp-Session-Id", session_id)
        .body(
            json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "method": "tools/call",
                "params": { "name": tool_name },
            })
            .to_string(),
        )
        .send()
        .await
        .expect("tools/call");
    assert!(resp.status().is_success(), "tools/call: {}", resp.status());
    parse_jsonrpc_body(resp).await
}

/// Some endpoints reply as plain JSON, others as a single-event SSE
/// stream — accept both.
pub async fn parse_jsonrpc_body(resp: reqwest::Response) -> Value {
    let body = resp.text().await.unwrap();
    if let Ok(v) = serde_json::from_str::<Value>(&body) {
        return v;
    }
    let data: String = body
        .lines()
        .filter_map(|l| l.strip_prefix("data: "))
        .collect::<Vec<_>>()
        .join("");
    serde_json::from_str(&data)
        .unwrap_or_else(|e| panic!("body parses as JSON or SSE: {e}\n{body}"))
}

/// Convenience constructor for a single text content block.
pub fn text_block(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

/// Pull `result.content` off a parsed JSON-RPC tools/call response.
pub fn content_blocks(rpc_response: &Value) -> &Vec<Value> {
    rpc_response["result"]["content"]
        .as_array()
        .expect("result.content is an array")
}
