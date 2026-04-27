//! Two upstreams advertising the **same** server_info.name — the proxy
//! must disambiguate via `<name>_<index>_<tool>`. A third upstream with
//! a unique name in the same session keeps the bare `<name>_<tool>` form.

mod common;

use std::collections::{HashMap, HashSet};

use common::{TestRig, UpstreamSpec};
use objectiveai_mcp_test_upstream::{TestTool, TestToolBehavior};

fn echo_tool(name: &str) -> TestTool {
    TestTool {
        name: name.into(),
        description: None,
        behavior: TestToolBehavior::Echo,
    }
}

#[tokio::test]
async fn duplicate_server_names_get_indexed() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("fs").with_tools(vec![echo_tool("Read")]),
        UpstreamSpec::new("fs").with_tools(vec![echo_tool("Read")]),
        UpstreamSpec::new("solo").with_tools(vec![echo_tool("Ping")]),
    ])
    .await;

    let mut headers = HashMap::new();
    headers.insert("X-MCP-Servers", rig.x_mcp_servers());
    let client = rig.connect_client(headers).await;

    let tools = client
        .peer()
        .list_all_tools()
        .await
        .expect("list_all_tools");

    let names: HashSet<String> = tools.iter().map(|t| t.name.clone().into()).collect();
    assert_eq!(
        names,
        HashSet::from([
            "fs_0_Read".to_string(),
            "fs_1_Read".to_string(),
            "solo_Ping".to_string(),
        ]),
        "duplicate-name upstreams must be indexed; uniquely-named upstream stays bare-prefixed",
    );

    client.cancel().await.ok();
}
