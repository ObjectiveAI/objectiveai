use super::*;
use crate::test_mcp_server::{self, TestTool};

fn mcp_tool(name: &str) -> objectiveai_sdk::mcp::tool::Tool {
    objectiveai_sdk::mcp::tool::Tool {
        name: name.into(),
        title: None,
        description: None,
        icons: None,
        input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
            r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
            properties: None,
            required: None,
            extra: indexmap::IndexMap::new(),
        },
        output_schema: None,
        annotations: None,
        execution: None,
        _meta: None,
    }
}

fn response_format_tool(name: &str) -> objectiveai_sdk::agent::completions::request::ResponseFormat {
    objectiveai_sdk::agent::completions::request::ResponseFormat::ToolCall {
        name: name.into(),
        description: "test".into(),
        schema: indexmap::IndexMap::new(),
        required: None,
    }
}

#[tokio::test]
async fn test_no_tools() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let (names, map) = resolve_tools(None, None).await.unwrap();
    assert!(names.is_empty());
    assert!(map.is_empty());
}

#[tokio::test]
async fn test_single_mcp_tool() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let server = test_mcp_server::spawn("alpha", vec![TestTool::noop(mcp_tool("search"))]).await;
    let conn = test_mcp_server::connect_through_proxy(&[&server]).await;

    // The proxy fans out tools/list across upstreams and prefixes every
    // tool name with `<upstream-server-name>_`. Our server identifies as
    // `alpha`, so the only tool comes back as `alpha_search`. The
    // original tool name is preserved on the resolved tool itself.
    let (names, map) = resolve_tools(Some(&conn), None).await.unwrap();
    assert_eq!(names, vec!["alpha_search"]);
    assert!(matches!(map.get("alpha_search"), Some(ResolvedTool::Mcp { tool, .. }) if tool.name == "alpha_search"));
}

#[tokio::test]
async fn test_single_response_format_tool() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let rf = response_format_tool("submit");
    let (names, map) = resolve_tools(None, Some(&rf)).await.unwrap();
    assert_eq!(names, vec!["submit"]);
    assert!(matches!(map.get("submit"), Some(ResolvedTool::ResponseFormat { .. })));
}

#[tokio::test]
async fn test_response_format_text_yields_no_tool() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let rf = objectiveai_sdk::agent::completions::request::ResponseFormat::Text;
    let (names, map) = resolve_tools(None, Some(&rf)).await.unwrap();
    assert!(names.is_empty());
    assert!(map.is_empty());
}

#[tokio::test]
async fn test_mcp_and_response_format() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let server = test_mcp_server::spawn("alpha", vec![
        TestTool::noop(mcp_tool("search")),
        TestTool::noop(mcp_tool("list")),
    ])
    .await;
    let conn = test_mcp_server::connect_through_proxy(&[&server]).await;
    let rf = response_format_tool("submit");

    // Single upstream → tools prefixed with `alpha_`. The response-format
    // tool keeps its bare name (it's local, not an MCP tool).
    let (names, map) = resolve_tools(Some(&conn), Some(&rf)).await.unwrap();
    assert_eq!(names.len(), 3);
    assert!(map.contains_key("alpha_search"));
    assert!(map.contains_key("alpha_list"));
    assert!(map.contains_key("submit"));
    assert!(matches!(map.get("submit"), Some(ResolvedTool::ResponseFormat { .. })));
    assert!(matches!(map.get("alpha_search"), Some(ResolvedTool::Mcp { .. })));
}

#[tokio::test]
async fn test_multi_upstream_via_proxy_returns_union() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    // The proxy fans out across multiple upstream MCP servers and prefixes
    // each tool with the upstream's server name. With distinct names we
    // get the bare-prefix form `<name>_<tool>` (no `_<index>` suffix —
    // suffixing only kicks in when two upstreams share a name).
    let server_a = test_mcp_server::spawn("alpha", vec![TestTool::noop(mcp_tool("search"))]).await;
    let server_b = test_mcp_server::spawn("beta", vec![TestTool::noop(mcp_tool("compile"))]).await;
    let conn = test_mcp_server::connect_through_proxy(&[&server_a, &server_b]).await;

    let (names, map) = resolve_tools(Some(&conn), None).await.unwrap();
    assert!(map.contains_key("alpha_search"), "missing 'search' from server_a; got {names:?}");
    assert!(map.contains_key("beta_compile"), "missing 'compile' from server_b; got {names:?}");
    assert_eq!(names.len(), 2);
}
