//! After a `tools/call` flushes the queue, the next `tools/call` gets
//! a clean response with no wrapper.

mod common;
mod notify_helpers;

use common::{TestRig, UpstreamSpec};
use notify_helpers::{call_tool, content_blocks, init_session, post_notify, text_block};
use serde_json::json;
use test_upstream::{TestTool, TestToolBehavior};

#[tokio::test]
async fn second_tool_call_after_flush_has_no_wrapper() {
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
        json!([text_block("flush-me")]),
    )
    .await;

    // First call drains the queue.
    let first = call_tool(&client, &rig.proxy.url, &session_id, 2, "alpha_say").await;
    let first_blocks = content_blocks(&first);
    assert_eq!(first_blocks.len(), 4);
    assert!(first_blocks[0]["text"]
        .as_str()
        .unwrap()
        .starts_with("<system-reminder>"));

    // Second call: no notify between calls, response should be the
    // bare upstream content with no wrapper bracket.
    let second = call_tool(&client, &rig.proxy.url, &session_id, 3, "alpha_say").await;
    let second_blocks = content_blocks(&second);
    assert_eq!(
        second_blocks.len(),
        1,
        "second call should be unwrapped, got {second_blocks:?}"
    );
    assert_eq!(second_blocks[0]["text"], "tool-output");
}
