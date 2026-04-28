//! Start a long-running tool call, fire `notifications/cancelled` for
//! it, and assert the call returns JSON-RPC -32800 well before the
//! tool's own sleep would have completed.
//!
//! Has to be raw HTTP — rmcp's high-level client wires `cancel` to the
//! request future's drop, which doesn't translate to the proxy's
//! `notifications/cancelled` path. We want to test the explicit
//! cancellation message, not connection-drop cancellation.

mod common;

use std::time::{Duration, Instant};

use common::{TestRig, UpstreamSpec};
use test_upstream::{TestTool, TestToolBehavior};
use reqwest::StatusCode;
use serde_json::Value;

#[tokio::test]
async fn notifications_cancelled_aborts_in_flight_tool_call() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("slow").with_tools(vec![TestTool {
            name: "wait".into(),
            description: None,
            behavior: TestToolBehavior::SleepThenEcho { duration_ms: 5_000 },
        }]),
    ])
    .await;
    let http = reqwest::Client::new();

    // 1. initialize.
    let init_resp = http
        .post(&rig.proxy.url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("X-MCP-Servers", rig.x_mcp_servers())
        .body(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{
                "protocolVersion":"2025-11-25",
                "capabilities":{},
                "clientInfo":{"name":"t","version":"0"}
            }}"#,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(init_resp.status(), StatusCode::OK);
    let session_id = init_resp
        .headers()
        .get("Mcp-Session-Id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let _: Value = init_resp.json().await.unwrap();

    // 2. Kick off the slow call (id = 42). Don't await yet.
    let call_url = rig.proxy.url.clone();
    let session_for_call = session_id.clone();
    let call_started = Instant::now();
    let call_future = tokio::spawn(async move {
        reqwest::Client::new()
            .post(&call_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("Mcp-Session-Id", &session_for_call)
            .body(
                r#"{"jsonrpc":"2.0","id":42,"method":"tools/call","params":{
                    "name":"slow_wait","arguments":{}
                }}"#,
            )
            .send()
            .await
            .unwrap()
    });

    // 3. Give the proxy ~150ms to register the in-flight token.
    tokio::time::sleep(Duration::from_millis(150)).await;

    // 4. Fire notifications/cancelled with the matching requestId.
    let cancel_resp = http
        .post(&rig.proxy.url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Mcp-Session-Id", &session_id)
        .body(
            r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{
                "requestId":42,"reason":"test cancel"
            }}"#,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(cancel_resp.status(), StatusCode::ACCEPTED);

    // 5. The original call should now return -32800, well within the
    //    5s the SleepThenEcho would otherwise wait.
    let resp = tokio::time::timeout(Duration::from_secs(2), call_future)
        .await
        .expect("call did not complete within 2s; cancellation didn't propagate")
        .expect("call task panicked");
    let elapsed = call_started.elapsed();
    assert!(
        elapsed < Duration::from_secs(2),
        "cancellation took {elapsed:?}; should be ~150ms",
    );

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["error"]["code"], -32800);
}
