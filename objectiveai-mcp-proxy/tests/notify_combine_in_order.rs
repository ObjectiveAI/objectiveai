//! Multiple `/notify` POSTs that arrive before the next `tools/call`
//! get concatenated, in arrival order, inside one wrapper.

mod common;
mod notify_helpers;

use common::{TestRig, UpstreamSpec};
use notify_helpers::{call_tool, content_blocks, init_session, post_notify, text_block};
use serde_json::json;
use test_upstream::{TestTool, TestToolBehavior};

#[tokio::test]
async fn multiple_notifies_combine_in_arrival_order() {
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
        json!([text_block("first")]),
    )
    .await;
    post_notify(
        &client,
        &rig.proxy.url,
        &session_id,
        json!([text_block("second"), text_block("third")]),
    )
    .await;
    post_notify(
        &client,
        &rig.proxy.url,
        &session_id,
        json!([text_block("fourth")]),
    )
    .await;

    let response = call_tool(&client, &rig.proxy.url, &session_id, 2, "alpha_say").await;
    let blocks = content_blocks(&response);

    // open + 4 queued + close + 1 tool output
    assert_eq!(blocks.len(), 7, "got {blocks:?}");
    assert_eq!(
        blocks[0]["text"],
        "<system-reminder>\nThe user sent a new message while you were working:\n",
    );
    assert_eq!(blocks[1]["text"], "first");
    assert_eq!(blocks[2]["text"], "second");
    assert_eq!(blocks[3]["text"], "third");
    assert_eq!(blocks[4]["text"], "fourth");
    assert_eq!(blocks[5]["text"], "\n\n</system-reminder>");
    assert_eq!(blocks[6]["text"], "tool-output");
}
