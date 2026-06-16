//! Routing-prefix scheme. The proxy keys each upstream by a `_`/`.`-free
//! prefix derived from its `server_info.name`, escalating only on collision:
//! `name` → `name-version` → `name-version-index`. Tools/resources are
//! shipped as `<prefix>_<original>` and inbound calls route by splitting on
//! the first `_`. (`name`/`version` have `_` and `.` normalized to `-`.)

mod common;

use std::collections::{HashMap, HashSet};

use common::{TestRig, UpstreamSpec};
use rmcp::model::CallToolRequestParams;
use test_upstream::{TestTool, TestToolBehavior};

fn echo_tool(name: &str) -> TestTool {
    TestTool {
        name: name.into(),
        description: None,
        behavior: TestToolBehavior::Echo,
    }
}

/// Spin up the given upstreams behind the proxy, list all tools through the
/// proxy, and return the set of (prefixed) tool names.
async fn listed_tool_names(specs: Vec<UpstreamSpec>) -> HashSet<String> {
    let rig = TestRig::new(specs).await;
    let mut headers = HashMap::new();
    headers.insert("X-MCP-Servers", rig.x_mcp_servers());
    let client = rig.connect_client(headers).await;

    let tools = client.peer().list_all_tools().await.expect("list_all_tools");
    let names: HashSet<String> = tools.iter().map(|t| t.name.clone().into()).collect();

    client.cancel().await.ok();
    names
}

/// Distinct names → bare `normalize(name)` prefix. `_` and `.` in the name
/// are normalized to `-` so the first `_` stays the prefix boundary.
#[tokio::test]
async fn distinct_names_use_bare_normalized_prefix() {
    let names = listed_tool_names(vec![
        UpstreamSpec::new("fs").with_tools(vec![echo_tool("Read")]),
        UpstreamSpec::new("my.web_search").with_tools(vec![echo_tool("Query")]),
    ])
    .await;

    assert_eq!(
        names,
        HashSet::from([
            "fs_Read".to_string(),
            "my-web-search_Query".to_string(),
        ]),
        "distinct names keep a bare, normalized prefix",
    );
}

/// Same name, different version → both escalate to `{name}-{version}`.
#[tokio::test]
async fn same_name_different_version_disambiguates_by_version() {
    let names = listed_tool_names(vec![
        UpstreamSpec::new("fs")
            .with_server_version("1.0")
            .with_tools(vec![echo_tool("Read")]),
        UpstreamSpec::new("fs")
            .with_server_version("2.0")
            .with_tools(vec![echo_tool("Read")]),
    ])
    .await;

    assert_eq!(
        names,
        HashSet::from([
            "fs-1-0_Read".to_string(),
            "fs-2-0_Read".to_string(),
        ]),
        "same name with differing versions disambiguates by normalized version",
    );
}

/// Same name AND version → both escalate to `{name}-{version}-{index}`.
/// Index is the url-sorted position, which the test can't predict from the
/// random ports — but both upstreams carry identical tools, so the SET of
/// prefixed names is stable either way.
#[tokio::test]
async fn same_name_same_version_disambiguates_by_index() {
    let names = listed_tool_names(vec![
        UpstreamSpec::new("fs")
            .with_server_version("9.9")
            .with_tools(vec![echo_tool("Read")]),
        UpstreamSpec::new("fs")
            .with_server_version("9.9")
            .with_tools(vec![echo_tool("Read")]),
    ])
    .await;

    assert_eq!(
        names,
        HashSet::from([
            "fs-9-9-0_Read".to_string(),
            "fs-9-9-1_Read".to_string(),
        ]),
        "identical name+version disambiguates by url-sorted index",
    );
}

/// A normalized prefix still routes `tools/call` to the owning upstream
/// with the un-prefixed tool name forwarded verbatim.
#[tokio::test]
async fn normalized_prefix_routes_tool_calls() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("my.fs").with_tools(vec![TestTool {
            name: "Read".into(),
            description: None,
            behavior: TestToolBehavior::Static {
                reply: "from-myfs".into(),
            },
        }]),
        UpstreamSpec::new("other_srv").with_tools(vec![TestTool {
            name: "Read".into(),
            description: None,
            behavior: TestToolBehavior::Static {
                reply: "from-other".into(),
            },
        }]),
    ])
    .await;

    let mut headers = HashMap::new();
    headers.insert("X-MCP-Servers", rig.x_mcp_servers());
    let client = rig.connect_client(headers).await;

    // `my.fs` → prefix `my-fs`; the original tool name `Read` is forwarded.
    let myfs = client
        .peer()
        .call_tool(CallToolRequestParams {
            name: "my-fs_Read".into(),
            arguments: None,
            meta: None,
            task: None,
        })
        .await
        .expect("call my-fs_Read");
    assert_eq!(first_text(&myfs), "from-myfs");

    // `other_srv` → prefix `other-srv`.
    let other = client
        .peer()
        .call_tool(CallToolRequestParams {
            name: "other-srv_Read".into(),
            arguments: None,
            meta: None,
            task: None,
        })
        .await
        .expect("call other-srv_Read");
    assert_eq!(first_text(&other), "from-other");

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
