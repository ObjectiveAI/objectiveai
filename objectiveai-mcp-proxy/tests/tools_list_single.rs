//! One upstream → tools come back as `<server-name>_<tool>`.

mod common;

use std::collections::{HashMap, HashSet};

use common::{TestRig, UpstreamSpec};
use test_upstream::{TestTool, TestToolBehavior};

#[tokio::test]
async fn single_upstream_tools_are_prefixed() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("fs").with_tools(vec![
            TestTool { name: "Read".into(), description: None, behavior: TestToolBehavior::Echo },
            TestTool { name: "Write".into(), description: None, behavior: TestToolBehavior::Echo },
        ]),
    ])
    .await;

    let mut headers = HashMap::new();
    headers.insert("X-MCP-Servers", rig.x_mcp_servers());
    let client = rig.connect_client(headers).await;

    let names: HashSet<String> = client
        .peer()
        .list_all_tools()
        .await
        .expect("list_all_tools")
        .into_iter()
        .map(|t| t.name.into())
        .collect();
    assert_eq!(
        names,
        HashSet::from(["fs_Read".to_string(), "fs_Write".to_string()])
    );

    client.cancel().await.ok();
}
