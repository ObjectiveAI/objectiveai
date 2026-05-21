//! `GET /notify/queued` — non-draining peek at the pending-notifications
//! queue. Returns a JSON boolean: `true` iff at least one block is
//! queued. The queue is left intact, so a subsequent `GET /notify`
//! (drain) still sees everything. Missing / unknown session ids return
//! 404 — same shape as the other `/notify` endpoints.

mod common;
mod notify_helpers;

use common::{TestRig, UpstreamSpec};
use notify_helpers::{get_notify, get_notify_queued, init_session, post_notify, text_block};
use reqwest::StatusCode;
use serde_json::{Value, json};
use test_upstream::{TestTool, TestToolBehavior};

#[tokio::test]
async fn get_notify_queued_reports_true_without_draining() {
    let rig = TestRig::new(vec![UpstreamSpec::new("alpha").with_tools(vec![
        TestTool {
            name: "say".into(),
            description: None,
            behavior: TestToolBehavior::Static {
                reply: "tool-output".into(),
            },
        },
    ])])
    .await;

    let client = reqwest::Client::new();
    let session_id = init_session(&client, &rig.proxy.url, &rig.x_mcp_servers()).await;

    // Empty queue → false.
    let resp = get_notify_queued(&client, &rig.proxy.url, &session_id).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let queued: bool = resp.json().await.expect("body is JSON boolean");
    assert!(!queued, "empty queue should report false");

    // Enqueue two blocks.
    post_notify(
        &client,
        &rig.proxy.url,
        &session_id,
        json!([text_block("first"), text_block("second")]),
    )
    .await;

    // Peek reports true but does NOT drain.
    let resp = get_notify_queued(&client, &rig.proxy.url, &session_id).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let queued: bool = resp.json().await.expect("body is JSON boolean");
    assert!(queued, "non-empty queue should report true");

    // A repeat peek still sees the queue — confirms non-draining.
    let resp = get_notify_queued(&client, &rig.proxy.url, &session_id).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let queued: bool = resp.json().await.expect("body is JSON boolean");
    assert!(queued, "second peek should still report true");

    // The actual drain still returns both blocks in order.
    let resp = get_notify(&client, &rig.proxy.url, &session_id).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let blocks: Vec<Value> = resp.json().await.expect("body is JSON array");
    assert_eq!(blocks.len(), 2, "drain should still see both blocks, got {blocks:?}");
    assert_eq!(blocks[0]["text"], "first");
    assert_eq!(blocks[1]["text"], "second");

    // After draining, peek reports false again.
    let resp = get_notify_queued(&client, &rig.proxy.url, &session_id).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let queued: bool = resp.json().await.expect("body is JSON boolean");
    assert!(!queued, "post-drain peek should report false");
}

#[tokio::test]
async fn get_notify_queued_unknown_session_returns_404() {
    let rig = TestRig::new(vec![UpstreamSpec::new("alpha").with_tools(vec![
        TestTool {
            name: "say".into(),
            description: None,
            behavior: TestToolBehavior::Static {
                reply: "tool-output".into(),
            },
        },
    ])])
    .await;

    let client = reqwest::Client::new();
    let resp = get_notify_queued(&client, &rig.proxy.url, "definitely-not-a-real-session").await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_notify_queued_missing_session_id_returns_404() {
    let rig = TestRig::new(vec![UpstreamSpec::new("alpha").with_tools(vec![
        TestTool {
            name: "say".into(),
            description: None,
            behavior: TestToolBehavior::Static {
                reply: "tool-output".into(),
            },
        },
    ])])
    .await;

    let client = reqwest::Client::new();
    let url = format!("{}notify/queued", rig.proxy.url);
    let resp = client
        .get(&url)
        .send()
        .await
        .expect("get /notify/queued without session header");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
