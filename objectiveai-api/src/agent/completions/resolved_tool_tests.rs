use std::sync::Arc;
use super::*;

fn mcp_tool(name: &str) -> objectiveai::mcp::tool::Tool {
    objectiveai::mcp::tool::Tool {
        name: name.into(),
        title: None,
        description: None,
        icons: None,
        input_schema: objectiveai::mcp::tool::ToolSchemaObject {
            r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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

fn invention_tool(name: &'static str) -> objectiveai::functions::inventions::InventionTool {
    objectiveai::functions::inventions::InventionTool::new_sync::<objectiveai::functions::inventions::EmptyObjectJsonSchema>(
        name,
        "test tool",
        |_| Ok("ok".into()),
    )
}

fn response_format_tool(name: &str) -> objectiveai::agent::completions::request::ResponseFormat {
    objectiveai::agent::completions::request::ResponseFormat::ToolCall {
        name: name.into(),
        description: "test".into(),
        schema: indexmap::IndexMap::new(),
        required: None,
    }
}

#[test]
fn test_no_tools() {
    let (names, map) = resolve_tools(&[], &[], None, None);
    assert!(names.is_empty());
    assert!(map.is_empty());
}

#[test]
fn test_single_mcp_tool() {
    let conn = objectiveai::mcp::Connection::new_for_test("server-a".into(), "https://a.com/mcp".into());
    let tools = Arc::new(vec![mcp_tool("search")]);
    let (names, map) = resolve_tools(&[conn], &[tools], None, None);
    assert_eq!(names, vec!["search"]);
    assert!(matches!(map.get("search"), Some(ResolvedTool::Mcp { tool, .. }) if tool.name == "search"));
}

#[test]
fn test_single_invention_tool() {
    let inv = invention_tool("execute");
    let (names, map) = resolve_tools(&[], &[], Some(&[inv]), None);
    assert_eq!(names, vec!["execute"]);
    assert!(matches!(map.get("execute"), Some(ResolvedTool::InventionTool(_))));
}

#[test]
fn test_single_response_format_tool() {
    let rf = response_format_tool("submit");
    let (names, map) = resolve_tools(&[], &[], None, Some(&rf));
    assert_eq!(names, vec!["submit"]);
    assert!(matches!(map.get("submit"), Some(ResolvedTool::ResponseFormat { .. })));
}

#[test]
fn test_response_format_text_yields_no_tool() {
    let rf = objectiveai::agent::completions::request::ResponseFormat::Text;
    let (names, map) = resolve_tools(&[], &[], None, Some(&rf));
    assert!(names.is_empty());
    assert!(map.is_empty());
}

#[test]
fn test_multiple_mcp_no_conflicts() {
    let conn1 = objectiveai::mcp::Connection::new_for_test("alpha".into(), "https://a.com/mcp".into());
    let conn2 = objectiveai::mcp::Connection::new_for_test("beta".into(), "https://b.com/mcp".into());
    let tools1 = Arc::new(vec![mcp_tool("search"), mcp_tool("list")]);
    let tools2 = Arc::new(vec![mcp_tool("compile"), mcp_tool("run")]);
    let (names, map) = resolve_tools(&[conn1, conn2], &[tools1, tools2], None, None);
    assert_eq!(names.len(), 4);
    for name in &["search", "list", "compile", "run"] {
        assert!(map.contains_key(*name), "missing {name}");
    }
}

#[test]
fn test_mcp_conflict_different_server_names() {
    let conn1 = objectiveai::mcp::Connection::new_for_test("alpha".into(), "https://a.com/mcp".into());
    let conn2 = objectiveai::mcp::Connection::new_for_test("beta".into(), "https://b.com/mcp".into());
    let tools1 = Arc::new(vec![mcp_tool("search")]);
    let tools2 = Arc::new(vec![mcp_tool("search")]);
    let (names, map) = resolve_tools(&[conn1, conn2], &[tools1, tools2], None, None);
    assert_eq!(names.len(), 2);
    assert!(map.contains_key("search (alpha)"));
    assert!(map.contains_key("search (beta)"));
}

#[test]
fn test_mcp_conflict_same_server_name_different_urls() {
    let conn1 = objectiveai::mcp::Connection::new_for_test("myserver".into(), "https://a.com/mcp".into());
    let conn2 = objectiveai::mcp::Connection::new_for_test("myserver".into(), "https://b.com/mcp".into());
    let tools1 = Arc::new(vec![mcp_tool("search")]);
    let tools2 = Arc::new(vec![mcp_tool("search")]);
    let (names, map) = resolve_tools(&[conn1, conn2], &[tools1, tools2], None, None);
    assert_eq!(names.len(), 2);
    assert!(map.contains_key("search (myserver(https://a.com/mcp))"));
    assert!(map.contains_key("search (myserver(https://b.com/mcp))"));
}

#[test]
fn test_mcp_conflict_same_server_name_unique_tool_not_suffixed() {
    // Two servers with the same name, conflicting on "search" but "list" is unique
    let conn1 = objectiveai::mcp::Connection::new_for_test("myserver".into(), "https://a.com/mcp".into());
    let conn2 = objectiveai::mcp::Connection::new_for_test("myserver".into(), "https://b.com/mcp".into());
    let tools1 = Arc::new(vec![mcp_tool("search"), mcp_tool("list")]);
    let tools2 = Arc::new(vec![mcp_tool("search")]);
    let (names, map) = resolve_tools(&[conn1, conn2], &[tools1, tools2], None, None);
    assert_eq!(names.len(), 3);
    assert!(map.contains_key("list"), "unique tool should not be suffixed");
    assert!(map.contains_key("search (myserver(https://a.com/mcp))"));
    assert!(map.contains_key("search (myserver(https://b.com/mcp))"));
}

#[test]
fn test_invention_conflicts_with_mcp() {
    let conn = objectiveai::mcp::Connection::new_for_test("alpha".into(), "https://a.com/mcp".into());
    let tools = Arc::new(vec![mcp_tool("execute")]);
    let inv = invention_tool("execute");
    let (names, map) = resolve_tools(&[conn], &[tools], Some(&[inv]), None);
    assert_eq!(names.len(), 2);
    assert!(map.contains_key("execute (alpha)"));
    assert!(map.contains_key("execute (objectiveai-invention)"));
}

#[test]
fn test_invention_conflicts_with_response_format() {
    let inv = invention_tool("submit");
    let rf = response_format_tool("submit");
    let (names, map) = resolve_tools(&[], &[], Some(&[inv]), Some(&rf));
    assert_eq!(names.len(), 2);
    assert!(map.contains_key("submit"), "response format keeps original name");
    assert!(matches!(map.get("submit"), Some(ResolvedTool::ResponseFormat { .. })));
    assert!(map.contains_key("submit (objectiveai-invention)"));
}

#[test]
fn test_mcp_conflicts_with_response_format() {
    let conn = objectiveai::mcp::Connection::new_for_test("alpha".into(), "https://a.com/mcp".into());
    let tools = Arc::new(vec![mcp_tool("submit")]);
    let rf = response_format_tool("submit");
    let (names, map) = resolve_tools(&[conn], &[tools], None, Some(&rf));
    assert_eq!(names.len(), 2);
    assert!(map.contains_key("submit"), "response format keeps original name");
    assert!(matches!(map.get("submit"), Some(ResolvedTool::ResponseFormat { .. })));
    assert!(map.contains_key("submit (alpha)"));
}

#[test]
fn test_four_way_conflict_mcp_x2_invention_response_format() {
    let conn1 = objectiveai::mcp::Connection::new_for_test("alpha".into(), "https://a.com/mcp".into());
    let conn2 = objectiveai::mcp::Connection::new_for_test("beta".into(), "https://b.com/mcp".into());
    let tools1 = Arc::new(vec![mcp_tool("render")]);
    let tools2 = Arc::new(vec![mcp_tool("render")]);
    let inv = invention_tool("render");
    let rf = response_format_tool("render");
    let (names, map) = resolve_tools(&[conn1, conn2], &[tools1, tools2], Some(&[inv]), Some(&rf));
    assert_eq!(names.len(), 4);
    assert!(map.contains_key("render"), "response format keeps original name");
    assert!(matches!(map.get("render"), Some(ResolvedTool::ResponseFormat { .. })));
    assert!(map.contains_key("render (alpha)"));
    assert!(map.contains_key("render (beta)"));
    assert!(map.contains_key("render (objectiveai-invention)"));
}

#[test]
fn test_four_way_conflict_same_server_name() {
    let conn1 = objectiveai::mcp::Connection::new_for_test("myserver".into(), "https://a.com/mcp".into());
    let conn2 = objectiveai::mcp::Connection::new_for_test("myserver".into(), "https://b.com/mcp".into());
    let tools1 = Arc::new(vec![mcp_tool("render")]);
    let tools2 = Arc::new(vec![mcp_tool("render")]);
    let inv = invention_tool("render");
    let rf = response_format_tool("render");
    let (names, map) = resolve_tools(&[conn1, conn2], &[tools1, tools2], Some(&[inv]), Some(&rf));
    assert_eq!(names.len(), 4);
    assert!(map.contains_key("render"), "response format keeps original name");
    assert!(map.contains_key("render (myserver(https://a.com/mcp))"));
    assert!(map.contains_key("render (myserver(https://b.com/mcp))"));
    assert!(map.contains_key("render (objectiveai-invention)"));
}

#[test]
fn test_multiple_invention_tools_no_conflicts() {
    let inv1 = invention_tool("execute");
    let inv2 = invention_tool("validate");
    let (names, map) = resolve_tools(&[], &[], Some(&[inv1, inv2]), None);
    assert_eq!(names.len(), 2);
    assert!(map.contains_key("execute"));
    assert!(map.contains_key("validate"));
}

#[test]
fn test_mcp_tool_name_preserved_in_resolved() {
    let conn = objectiveai::mcp::Connection::new_for_test("alpha".into(), "https://a.com/mcp".into());
    let conn2 = objectiveai::mcp::Connection::new_for_test("beta".into(), "https://b.com/mcp".into());
    let tools = Arc::new(vec![mcp_tool("search")]);
    let tools2 = Arc::new(vec![mcp_tool("search")]);
    let (_, map) = resolve_tools(&[conn, conn2], &[tools, tools2], None, None);
    // Even though the resolved name has a suffix, the original tool_name is preserved
    if let Some(ResolvedTool::Mcp { tool, .. }) = map.get("search (alpha)") {
        assert_eq!(tool.name, "search");
    } else {
        panic!("expected Mcp variant for 'search (alpha)'");
    }
}

#[test]
fn test_mixed_no_conflicts() {
    let conn = objectiveai::mcp::Connection::new_for_test("server".into(), "https://s.com/mcp".into());
    let tools = Arc::new(vec![mcp_tool("search"), mcp_tool("list")]);
    let inv = invention_tool("execute");
    let rf = response_format_tool("submit");
    let (names, map) = resolve_tools(&[conn], &[tools], Some(&[inv]), Some(&rf));
    assert_eq!(names.len(), 4);
    // All names should be unsuffixed since no conflicts
    assert!(map.contains_key("search"));
    assert!(map.contains_key("list"));
    assert!(map.contains_key("execute"));
    assert!(map.contains_key("submit"));
}
