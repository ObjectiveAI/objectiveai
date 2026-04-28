use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::{Value, json};

use super::*;
use objectiveai::functions::inventions::InventionTool;

fn echo_tool() -> InventionTool {
    InventionTool {
        name: "echo",
        description: "Echoes back the input",
        parameters: {
            let mut m = IndexMap::new();
            m.insert(
                "text".to_string(),
                json!({ "type": "string", "description": "Text to echo" }),
            );
            m
        },
        call: Arc::new(|args| {
            Box::pin(async move {
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(empty)");
                Ok(text.to_string())
            })
        }),
    }
}

fn failing_tool() -> InventionTool {
    InventionTool {
        name: "fail",
        description: "Always fails",
        parameters: IndexMap::new(),
        call: Arc::new(|_| Box::pin(async { Err("something went wrong".to_string()) })),
    }
}

const ACCEPT: &str = "application/json, text/event-stream";

fn init_params() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "0.1.0" }
        }
    })
}

/// Parse a response that may be JSON or SSE (text/event-stream with `data:` lines).
async fn parse_response(resp: reqwest::Response) -> Value {
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
        .unwrap_or_else(|e| panic!("failed to parse response as JSON or SSE: {e}\nbody: {body}"))
}

/// Send initialize + notifications/initialized, return the session ID header.
async fn init_session(client: &reqwest::Client, base_url: &str) -> String {
    let resp = client
        .post(base_url)
        .header("Accept", ACCEPT)
        .json(&init_params())
        .send()
        .await
        .unwrap();
    let session_id = resp
        .headers()
        .get("mcp-session-id")
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    let _body = parse_response(resp).await;

    client
        .post(base_url)
        .header("Accept", ACCEPT)
        .header("mcp-session-id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .send()
        .await
        .unwrap();

    session_id
}

/// Send a JSON-RPC request with session, parse the response.
async fn rpc(client: &reqwest::Client, url: &str, session_id: &str, body: Value) -> Value {
    let resp = client
        .post(url)
        .header("Accept", ACCEPT)
        .header("mcp-session-id", session_id)
        .json(&body)
        .send()
        .await
        .unwrap();
    parse_response(resp).await
}

#[tokio::test]
async fn test_initialize() {
    let server = InventionServer::new(vec![]).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(&server.url())
        .header("Accept", ACCEPT)
        .json(&init_params())
        .send()
        .await
        .unwrap();
    let resp = parse_response(resp).await;

    assert!(resp["result"]["protocolVersion"].is_string());
    assert!(resp["result"]["serverInfo"].is_object());
}

#[tokio::test]
async fn test_notifications_initialized() {
    let server = InventionServer::new(vec![]).await;
    let client = reqwest::Client::new();
    let session_id = init_session(&client, &server.url()).await;
    assert!(!session_id.is_empty());
}

#[tokio::test]
async fn test_tools_list() {
    let server = InventionServer::new(vec![echo_tool()]).await;
    let client = reqwest::Client::new();
    let url = server.url();
    let session_id = init_session(&client, &url).await;

    let resp = rpc(
        &client,
        &url,
        &session_id,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    )
    .await;

    let tools = resp["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "echo");
    assert_eq!(tools[0]["description"], "Echoes back the input");
}

#[tokio::test]
async fn test_tools_call_success() {
    let server = InventionServer::new(vec![echo_tool()]).await;
    let client = reqwest::Client::new();
    let url = server.url();
    let session_id = init_session(&client, &url).await;

    let resp = rpc(
        &client,
        &url,
        &session_id,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": { "text": "hello world" }
            }
        }),
    )
    .await;

    assert_eq!(resp["result"]["isError"], false);
    assert_eq!(resp["result"]["content"][0]["type"], "text");
    assert_eq!(resp["result"]["content"][0]["text"], "hello world");
}

#[tokio::test]
async fn test_tools_call_error() {
    let server = InventionServer::new(vec![failing_tool()]).await;
    let client = reqwest::Client::new();
    let url = server.url();
    let session_id = init_session(&client, &url).await;

    let resp = rpc(
        &client,
        &url,
        &session_id,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "fail",
                "arguments": {}
            }
        }),
    )
    .await;

    assert_eq!(resp["result"]["isError"], true);
    assert_eq!(
        resp["result"]["content"][0]["text"],
        "something went wrong"
    );
}

#[tokio::test]
async fn test_tools_call_not_found() {
    let server = InventionServer::new(vec![]).await;
    let client = reqwest::Client::new();
    let url = server.url();
    let session_id = init_session(&client, &url).await;

    let resp = rpc(
        &client,
        &url,
        &session_id,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "nonexistent",
                "arguments": {}
            }
        }),
    )
    .await;

    assert!(resp["error"].is_object());
    assert!(resp["error"]["code"].as_i64().unwrap() < 0);
}

#[tokio::test]
async fn test_url() {
    let server = InventionServer::new(vec![]).await;
    let url = server.url();
    assert!(url.starts_with("http://127.0.0.1:"));
    assert!(url.ends_with("/mcp"));
}

#[tokio::test]
async fn test_unknown_method() {
    let server = InventionServer::new(vec![]).await;
    let client = reqwest::Client::new();
    let url = server.url();
    let session_id = init_session(&client, &url).await;

    let resp = rpc(
        &client,
        &url,
        &session_id,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "unknown/method",
            "params": {}
        }),
    )
    .await;

    assert!(resp["error"].is_object());
    assert!(resp["error"]["code"].as_i64().unwrap() < 0);
}
