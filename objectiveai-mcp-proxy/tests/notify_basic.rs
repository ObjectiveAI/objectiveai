//! Single `/notify` POST → next `tools/call` response is prepended with
//! one `<system-reminder>` wrapper bracketing the queued content block.

mod common;
mod notify_helpers;

use common::{TestRig, UpstreamSpec};
use notify_helpers::{call_tool, content_blocks, init_session, post_notify, text_block};
use reqwest::StatusCode;
use serde_json::json;
use test_upstream::{TestTool, TestToolBehavior};

#[tokio::test]
async fn notify_prepends_system_reminder_wrapper() {
    let rig = TestRig::new(vec![UpstreamSpec::new("alpha").with_tools(vec![
        TestTool {
            name: "say".into(),
            description: None,
            behavior: TestToolBehavior::Static {
                reply: "from-alpha".into(),
            },
        },
    ])])
    .await;

    let client = reqwest::Client::new();
    let session_id = init_session(&client, &rig.proxy.url, &rig.x_mcp_servers()).await;

    let resp = post_notify(
        &client,
        &rig.proxy.url,
        &session_id,
        json!([text_block("hello world")]),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let response = call_tool(&client, &rig.proxy.url, &session_id, 2, "alpha_say").await;
    let blocks = content_blocks(&response);
    assert_eq!(
        blocks.len(),
        4,
        "expected wrapper open + queued + wrapper close + tool output, got {blocks:?}"
    );
    assert_eq!(blocks[0]["type"], "text");
    assert_eq!(
        blocks[0]["text"],
        "<system-reminder>\nThe user sent a new message while you were working:\n",
    );
    assert_eq!(blocks[1]["type"], "text");
    assert_eq!(blocks[1]["text"], "hello world");
    assert_eq!(blocks[2]["type"], "text");
    assert_eq!(blocks[2]["text"], "\n\n</system-reminder>");
    assert_eq!(blocks[3]["type"], "text");
    assert_eq!(blocks[3]["text"], "from-alpha");
}
