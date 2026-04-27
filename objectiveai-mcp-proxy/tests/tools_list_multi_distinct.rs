//! Two upstreams with distinct names → both prefix sets concatenated.

mod common;

use std::collections::{HashMap, HashSet};

use common::{TestRig, UpstreamSpec};
use objectiveai_mcp_test_upstream::{TestTool, TestToolBehavior};

fn echo(name: &str) -> TestTool {
    TestTool { name: name.into(), description: None, behavior: TestToolBehavior::Echo }
}

#[tokio::test]
async fn distinct_upstream_names_get_their_own_prefix() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("alpha").with_tools(vec![echo("Read"), echo("Write")]),
        UpstreamSpec::new("beta").with_tools(vec![echo("Ping")]),
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
        HashSet::from([
            "alpha_Read".to_string(),
            "alpha_Write".to_string(),
            "beta_Ping".to_string(),
        ])
    );

    client.cancel().await.ok();
}
