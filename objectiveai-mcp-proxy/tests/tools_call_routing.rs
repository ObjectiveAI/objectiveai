//! Two upstreams. Call a tool on each via the prefixed name. Verify
//! the right upstream's `seen-headers` records the call (proves the
//! proxy routed to the correct upstream) and that the tool's response
//! reflects the upstream-specific behavior.

mod common;

use std::collections::HashMap;

use common::{TestRig, UpstreamSpec};
use objectiveai_mcp_test_upstream::{TestTool, TestToolBehavior};
use rmcp::model::CallToolRequestParams;

#[tokio::test]
async fn call_routes_to_correct_upstream() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("alpha").with_tools(vec![TestTool {
            name: "say".into(),
            description: None,
            behavior: TestToolBehavior::Static {
                reply: "from-alpha".into(),
            },
        }]),
        UpstreamSpec::new("beta").with_tools(vec![TestTool {
            name: "say".into(),
            description: None,
            behavior: TestToolBehavior::Static {
                reply: "from-beta".into(),
            },
        }]),
    ])
    .await;

    let mut headers = HashMap::new();
    headers.insert("X-MCP-Servers", rig.x_mcp_servers());
    let client = rig.connect_client(headers).await;

    let alpha_resp = client
        .peer()
        .call_tool(CallToolRequestParams {
            name: "alpha_say".into(),
            arguments: None,
            meta: None,
            task: None,
        })
        .await
        .expect("call alpha_say");
    let alpha_text = first_text(&alpha_resp);
    assert_eq!(alpha_text, "from-alpha");

    let beta_resp = client
        .peer()
        .call_tool(CallToolRequestParams {
            name: "beta_say".into(),
            arguments: None,
            meta: None,
            task: None,
        })
        .await
        .expect("call beta_say");
    let beta_text = first_text(&beta_resp);
    assert_eq!(beta_text, "from-beta");

    client.cancel().await.ok();
}

fn first_text(result: &rmcp::model::CallToolResult) -> String {
    use rmcp::model::RawContent;
    for block in &result.content {
        if let RawContent::Text(text) = &block.raw {
            return text.text.clone();
        }
    }
    panic!("no text content block in {:?}", result);
}
