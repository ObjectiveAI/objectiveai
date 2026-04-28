//! Posting `/notify` against an unknown / missing `Mcp-Session-Id`
//! returns 404 and leaves no global side effects (a real session can
//! still go through its tool calls without a stray wrapper).

mod common;
mod notify_helpers;

use common::{TestRig, UpstreamSpec};
use notify_helpers::{call_tool, content_blocks, init_session, post_notify, text_block};
use reqwest::StatusCode;
use serde_json::json;
use test_upstream::{TestTool, TestToolBehavior};

#[tokio::test]
async fn notify_unknown_session_returns_404_and_no_side_effect() {
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

    // POST /notify with a session id we never minted.
    let resp = post_notify(
        &client,
        &rig.proxy.url,
        "definitely-not-a-real-session",
        json!([text_block("ghost")]),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Now mint a real session and confirm its first tool call has the
    // bare upstream content — the bogus notify above didn't somehow
    // poison shared state.
    let session_id = init_session(&client, &rig.proxy.url, &rig.x_mcp_servers()).await;
    let response = call_tool(&client, &rig.proxy.url, &session_id, 2, "alpha_say").await;
    let blocks = content_blocks(&response);
    assert_eq!(blocks.len(), 1, "expected a single bare block, got {blocks:?}");
    assert_eq!(blocks[0]["text"], "tool-output");
}
