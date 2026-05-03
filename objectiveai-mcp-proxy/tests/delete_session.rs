//! initialize → DELETE → POST returns 404.
//!
//! We have to drive this with raw reqwest because rmcp's
//! `RunningService::cancel` doesn't fire a DELETE — it just tears down
//! the local task.

mod common;

use common::TestRig;
use reqwest::StatusCode;
use serde_json::Value;

#[tokio::test]
async fn delete_then_post_returns_404() {
    let rig = TestRig::new(vec![]).await;
    let http = reqwest::Client::new();

    // 1. initialize → grab session id from the response header.
    let init_resp = http
        .post(&rig.proxy.url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{
                "protocolVersion":"2025-06-18",
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
        .expect("Mcp-Session-Id on initialize response")
        .to_str()
        .unwrap()
        .to_string();
    let _: Value = init_resp.json().await.unwrap();

    // 2. DELETE → 200.
    let del_resp = http
        .delete(&rig.proxy.url)
        .header("Mcp-Session-Id", &session_id)
        .send()
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::OK);

    // 3. POST with the now-dead session id → 404.
    let post_resp = http
        .post(&rig.proxy.url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Mcp-Session-Id", &session_id)
        .body(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(post_resp.status(), StatusCode::NOT_FOUND);

    // 4. DELETE again on the same id → 404 (already gone).
    let del2_resp = http
        .delete(&rig.proxy.url)
        .header("Mcp-Session-Id", &session_id)
        .send()
        .await
        .unwrap();
    assert_eq!(del2_resp.status(), StatusCode::NOT_FOUND);
}
