//! tools/call (clean) → 2× /notify → tools/call (carries both notifies
//! combined, in arrival order, inside one wrapper).
//!
//! Distinct from `notify_drain.rs` (which asserts the *post*-drain
//! call is unwrapped) and `notify_combine_in_order.rs` (which only
//! exercises notifies before the *first* call). This one verifies the
//! enqueue → drain → enqueue cycle behaves cleanly across tool calls.

mod common;
mod notify_helpers;

use common::{TestRig, UpstreamSpec};
use notify_helpers::{call_tool, content_blocks, init_session, post_notify, text_block};
use serde_json::json;
use test_upstream::{TestTool, TestToolBehavior};

#[tokio::test]
async fn notifies_between_calls_are_combined_on_the_following_call() {
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

    // First call — no notifies queued, response is bare.
    let first = call_tool(&client, &rig.proxy.url, &session_id, 2, "alpha_say").await;
    let first_blocks = content_blocks(&first);
    assert_eq!(first_blocks.len(), 1, "first call should be bare, got {first_blocks:?}");
    assert_eq!(first_blocks[0]["text"], "tool-output");

    // Two notifies between the calls.
    post_notify(
        &client,
        &rig.proxy.url,
        &session_id,
        json!([text_block("between-one")]),
    )
    .await;
    post_notify(
        &client,
        &rig.proxy.url,
        &session_id,
        json!([text_block("between-two")]),
    )
    .await;

    // Second call — both notifies combined inside one wrapper, then the
    // bare tool output, in arrival order.
    let second = call_tool(&client, &rig.proxy.url, &session_id, 3, "alpha_say").await;
    let blocks = content_blocks(&second);
    assert_eq!(blocks.len(), 5, "got {blocks:?}");
    assert_eq!(
        blocks[0]["text"],
        "<system-reminder>\nThe user sent a new message while you were working:\n",
    );
    assert_eq!(blocks[1]["text"], "between-one");
    assert_eq!(blocks[2]["text"], "between-two");
    assert_eq!(blocks[3]["text"], "\n\n</system-reminder>");
    assert_eq!(blocks[4]["text"], "tool-output");
}
