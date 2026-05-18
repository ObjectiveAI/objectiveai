//! `GET /notify` atomically drains the pending-notifications queue and
//! returns the queued blocks as a JSON array. A second drain (with no
//! intervening `POST /notify`) returns `[]`. Missing / unknown session
//! ids return 404 — same shape as `POST /notify`.

mod common;
mod notify_helpers;

use common::{TestRig, UpstreamSpec};
use notify_helpers::{call_tool, content_blocks, get_notify, init_session, post_notify, text_block};
use reqwest::StatusCode;
use serde_json::{Value, json};
use test_upstream::{TestTool, TestToolBehavior};

#[tokio::test]
async fn get_notify_returns_then_drains_queue() {
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

    post_notify(
        &client,
        &rig.proxy.url,
        &session_id,
        json!([text_block("first"), text_block("second")]),
    )
    .await;

    // First GET drains the queue and returns both blocks in insertion order.
    let resp = get_notify(&client, &rig.proxy.url, &session_id).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let blocks: Vec<Value> = resp.json().await.expect("body is JSON array");
    assert_eq!(blocks.len(), 2, "expected two blocks, got {blocks:?}");
    assert_eq!(blocks[0]["type"], "text");
    assert_eq!(blocks[0]["text"], "first");
    assert_eq!(blocks[1]["type"], "text");
    assert_eq!(blocks[1]["text"], "second");

    // Second GET returns an empty array — the first drained the queue.
    let resp = get_notify(&client, &rig.proxy.url, &session_id).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let blocks: Vec<Value> = resp.json().await.expect("body is JSON array");
    assert!(blocks.is_empty(), "second drain should be empty, got {blocks:?}");

    // A subsequent tools/call must not see a leftover wrapper — confirm
    // the drain consumed the queue entirely (not just hid it from GET).
    let response = call_tool(&client, &rig.proxy.url, &session_id, 2, "alpha_say").await;
    let blocks = content_blocks(&response);
    assert_eq!(blocks.len(), 1, "expected bare tool output, got {blocks:?}");
    assert_eq!(blocks[0]["text"], "tool-output");
}

#[tokio::test]
async fn get_notify_unknown_session_returns_404() {
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

    let resp = get_notify(&client, &rig.proxy.url, "definitely-not-a-real-session").await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_notify_missing_session_id_returns_404() {
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
    let notify_url = format!("{}notify", rig.proxy.url);
    let resp = client
        .get(&notify_url)
        .send()
        .await
        .expect("get /notify without session header");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
