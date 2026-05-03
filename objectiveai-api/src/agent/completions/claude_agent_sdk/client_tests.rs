use objectiveai::agent::completions::request::AgentCompletionCreateParams;
use objectiveai::agent::claude_agent_sdk::{Agent, AgentBase};
use objectiveai::agent::{InlineAgentBase, InlineAgentBaseWithFallbacks, InlineAgentBaseWithFallbacksOrRemoteCommitOptional};

use super::Client;
use crate::agent::completions::upstream_client::UpstreamClient;
use crate::test_mcp_server::{self, TestTool};

fn default_client() -> Client {
    Client::new(String::new(), true, 0, 180, 1)
}

fn default_agent() -> Agent {
    Agent::try_from(AgentBase {
        model: "test-model".into(),
        ..Default::default()
    })
    .unwrap()
}

fn default_params() -> AgentCompletionCreateParams {
    AgentCompletionCreateParams {
        messages: vec![],
        agent: InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            InlineAgentBaseWithFallbacks {
                inner: InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    }
}

fn make_mcp_tool(name: &str) -> objectiveai::mcp::tool::Tool {
    objectiveai::mcp::tool::Tool {
        name: name.into(),
        title: None,
        description: Some(format!("{name} tool")),
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

#[tokio::test]
async fn test_tools_not_allowed_with_tools_present() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let client = default_client();
    let agent = default_agent();
    let params = default_params();
    let server = test_mcp_server::spawn("test", vec![TestTool::noop(make_mcp_tool("some_tool"))]).await;
    let conn = test_mcp_server::connect_through_proxy(&[&server]).await;

    let result = client
        .create(
            "test", 1000, &agent, None, &params, &[], Some(conn), None, None,
            rust_decimal::Decimal::ONE, false, None, None, None, None,
        )
        .await;
    match result {
        Err(super::Error::ToolsNotAllowed) => {}
        Err(e) => panic!("expected ToolsNotAllowed, got {e}"),
        Ok(_) => panic!("expected error"),
    }
}

#[tokio::test]
async fn test_tools_not_allowed_without_tools_proceeds() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let client = default_client();
    let agent = default_agent();
    let params = default_params();

    // With no tools, tools_enabled = false should not cause ToolsNotAllowed.
    // It will fail for other reasons (no SDK installed), but NOT with ToolsNotAllowed.
    let result = client
        .create(
            "test", 1000, &agent, None, &params, &[], None, None, None,
            rust_decimal::Decimal::ONE, false, None, None, None, None,
        )
        .await;
    match result {
        Err(super::Error::ToolsNotAllowed) => {
            panic!("should not get ToolsNotAllowed when no tools are present")
        }
        _ => {} // any other result is fine
    }
}
