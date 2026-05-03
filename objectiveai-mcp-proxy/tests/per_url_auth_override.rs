//! Two upstreams, each requiring a *different* `Authorization` value.
//! `X-MCP-Headers: {<urlA>: {"Authorization": "Bearer A"}, <urlB>: {"Authorization": "Bearer B"}}`
//! lets both connect — the proxy applies each per-URL header set on
//! every request to that upstream.

mod common;

use std::collections::{HashMap, HashSet};

use common::{TestRig, UpstreamSpec};
use test_upstream::{TestTool, TestToolBehavior};

fn echo(name: &str) -> TestTool {
    TestTool { name: name.into(), description: None, behavior: TestToolBehavior::Echo }
}

#[tokio::test]
async fn per_url_authorization_overrides_apply() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("alpha")
            .with_tools(vec![echo("aTool")])
            .with_require_auth("Bearer A"),
        UpstreamSpec::new("beta")
            .with_tools(vec![echo("bTool")])
            .with_require_auth("Bearer B"),
    ])
    .await;

    let per_url_headers = serde_json::json!({
        rig.upstreams[0].url.clone(): { "Authorization": "Bearer A" },
        rig.upstreams[1].url.clone(): { "Authorization": "Bearer B" },
    });
    let mut headers = HashMap::new();
    headers.insert("X-MCP-Servers", rig.x_mcp_servers());
    headers.insert("X-MCP-Headers", per_url_headers.to_string());
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
        HashSet::from(["alpha_aTool".to_string(), "beta_bTool".to_string()]),
    );

    client.cancel().await.ok();
}
