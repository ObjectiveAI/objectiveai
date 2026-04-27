use std::collections::HashMap;

use objectiveai::agent::completions::request::AgentCompletionCreateParams;
use objectiveai::agent::claude_agent_sdk::{Agent, AgentBase};
use objectiveai::agent::{InlineAgentBase, InlineAgentBaseWithFallbacks, InlineAgentBaseWithFallbacksOrRemoteCommitOptional};

use super::Client;
use crate::agent::completions::upstream_client::UpstreamClient;

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

#[tokio::test]
async fn test_tools_not_allowed_with_tools_present() {
    let client = default_client();
    let agent = default_agent();
    let params = default_params();
    let tool_names = vec!["some_tool".into()];
    let mut tool_map = HashMap::new();
    tool_map.insert(
        "some_tool".into(),
        crate::agent::completions::tool::ResolvedTool::ResponseFormat {
            description: "test".into(),
            schema: indexmap::IndexMap::new(),
        },
    );

    let result = client
        .create(
            "test", 1000, &agent, None, &params, &[], &[], None,
            &tool_names, &tool_map, None, None,
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
    let client = default_client();
    let agent = default_agent();
    let params = default_params();

    // With no tools, tools_enabled = false should not cause ToolsNotAllowed.
    // It will fail for other reasons (no SDK installed), but NOT with ToolsNotAllowed.
    let result = client
        .create(
            "test", 1000, &agent, None, &params, &[], &[], None,
            &[], &HashMap::new(), None, None,
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
