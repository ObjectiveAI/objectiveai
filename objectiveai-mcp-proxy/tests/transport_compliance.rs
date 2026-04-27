//! Raw-HTTP edge cases on the proxy's POST endpoint:
//! - missing Accept header → 406 Not Acceptable
//! - malformed JSON body → 400 + JSON-RPC -32700 envelope
//! - missing Mcp-Session-Id (after initialize) → 404
//! - unknown Mcp-Session-Id → 404
//!
//! Bypasses rmcp entirely — these are transport-level checks. Uses
//! reqwest directly so we can construct exactly the bad requests the
//! spec says to reject.

mod common;

use common::TestRig;
use reqwest::StatusCode;
use serde_json::Value;

#[tokio::test]
async fn post_with_insufficient_accept_returns_406() {
    // Reqwest auto-injects `Accept: */*` if you don't set one (which we
    // accept as a wildcard). To exercise the rejection path we have to
    // send an Accept that *only* lists something the proxy doesn't
    // satisfy — `text/html` is a fine stand-in for "wrong content type."
    let rig = TestRig::new(vec![]).await;
    let resp = reqwest::Client::new()
        .post(&rig.proxy.url)
        .header("Content-Type", "application/json")
        .header("Accept", "text/html")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
}

#[tokio::test]
async fn malformed_json_returns_400_with_jsonrpc_envelope() {
    let rig = TestRig::new(vec![]).await;
    let resp = reqwest::Client::new()
        .post(&rig.proxy.url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body("{not json")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: Value = resp.json().await.expect("body is JSON-RPC envelope");
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["error"]["code"], -32700);
}

#[tokio::test]
async fn missing_session_id_returns_404() {
    let rig = TestRig::new(vec![]).await;
    let resp = reqwest::Client::new()
        .post(&rig.proxy.url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unknown_session_id_returns_404() {
    let rig = TestRig::new(vec![]).await;
    let resp = reqwest::Client::new()
        .post(&rig.proxy.url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Mcp-Session-Id", "definitely-not-a-real-session")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn wrong_protocol_version_returns_jsonrpc_invalid_request() {
    let rig = TestRig::new(vec![]).await;
    let resp = reqwest::Client::new()
        .post(&rig.proxy.url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{
                "protocolVersion":"1990-01-01",
                "capabilities":{},
                "clientInfo":{"name":"t","version":"0"}
            }}"#,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["error"]["code"], -32600);
}
