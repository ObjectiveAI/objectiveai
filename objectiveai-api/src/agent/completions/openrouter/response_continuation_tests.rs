use super::super::{ContinuationItem, UpstreamClient};
use objectiveai_sdk::agent::completions::message::*;

fn make_client() -> super::Client {
    super::Client::new(
        reqwest::Client::new(),
        String::new(),
        None,
        String::new(),
        String::new(),
        String::new(),
    )
}

#[test]
fn test_empty_messages_no_continuation() {
    let client = make_client();
    let result = client.response_continuation(
        indexmap::IndexMap::new(),
        None,
        &[],
        None,
    );
    assert_eq!(result, objectiveai_sdk::agent::openrouter::Continuation {
        upstream: objectiveai_sdk::agent::openrouter::Upstream::Openrouter,
        agent_instance_hierarchy: String::new(),
        messages: vec![],
        mcp_sessions: indexmap::IndexMap::new(),
        ws_session_id: None,
    });
}

#[test]
fn test_messages_only() {
    let client = make_client();
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("Hello".into()),
            name: None,
        }),
    ];
    let result = client.response_continuation(
        indexmap::IndexMap::new(),
        None,
        &messages,
        None,
    );
    assert_eq!(result, objectiveai_sdk::agent::openrouter::Continuation {
        upstream: objectiveai_sdk::agent::openrouter::Upstream::Openrouter,
        agent_instance_hierarchy: String::new(),
        messages: vec![
            Message::User(UserMessage {
                content: RichContent::Text("Hello".into()),
                name: None,
            }),
        ],
        mcp_sessions: indexmap::IndexMap::new(),
        ws_session_id: None,
    });
}

#[test]
fn test_messages_with_continuation() {
    let client = make_client();
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("Hello".into()),
            name: None,
        }),
    ];
    let continuation = vec![
        ContinuationItem::State(AssistantMessage {
            content: Some(RichContent::Text("Hi there".into())),
            name: None, refusal: None, tool_calls: None, reasoning: None,
        }),
        ContinuationItem::UserMessage(UserMessage {
            content: RichContent::Text("Follow up".into()),
            name: None,
        }),
    ];
    let result = client.response_continuation(
        indexmap::IndexMap::new(),
        None,
        &messages,
        Some(&continuation),
    );
    assert_eq!(result, objectiveai_sdk::agent::openrouter::Continuation {
        upstream: objectiveai_sdk::agent::openrouter::Upstream::Openrouter,
        agent_instance_hierarchy: String::new(),
        messages: vec![
            Message::User(UserMessage {
                content: RichContent::Text("Hello".into()),
                name: None,
            }),
            Message::Assistant(AssistantMessage {
                content: Some(RichContent::Text("Hi there".into())),
                name: None, refusal: None, tool_calls: None, reasoning: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("Follow up".into()),
                name: None,
            }),
        ],
        mcp_sessions: indexmap::IndexMap::new(),
        ws_session_id: None,
    });
}

#[test]
fn test_request_continuation_messages_come_first() {
    let client = make_client();
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("Current turn".into()),
            name: None,
        }),
    ];
    let rc = objectiveai_sdk::agent::openrouter::Continuation {
        upstream: objectiveai_sdk::agent::openrouter::Upstream::Openrouter,
        agent_instance_hierarchy: String::new(),
        messages: vec![
            Message::User(UserMessage {
                content: RichContent::Text("Previous turn".into()),
                name: None,
            }),
            Message::Assistant(AssistantMessage {
                content: Some(RichContent::Text("Previous response".into())),
                name: None, refusal: None, tool_calls: None, reasoning: None,
            }),
        ],
        mcp_sessions: indexmap::IndexMap::new(),
        ws_session_id: None,
    };
    let result = client.response_continuation(
        indexmap::IndexMap::new(),
        Some(&rc),
        &messages,
        None,
    );
    assert_eq!(result, objectiveai_sdk::agent::openrouter::Continuation {
        upstream: objectiveai_sdk::agent::openrouter::Upstream::Openrouter,
        agent_instance_hierarchy: String::new(),
        messages: vec![
            Message::User(UserMessage {
                content: RichContent::Text("Previous turn".into()),
                name: None,
            }),
            Message::Assistant(AssistantMessage {
                content: Some(RichContent::Text("Previous response".into())),
                name: None, refusal: None, tool_calls: None, reasoning: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("Current turn".into()),
                name: None,
            }),
        ],
        mcp_sessions: indexmap::IndexMap::new(),
        ws_session_id: None,
    });
}

#[test]
fn test_mcp_sessions_preserved() {
    let client = make_client();
    let mut mcp_sessions = indexmap::IndexMap::new();
    mcp_sessions.insert("http://mcp.example.com".into(), "sess-abc".into());
    let result = client.response_continuation(
        mcp_sessions.clone(),
        None,
        &[],
        None,
    );
    assert_eq!(result.mcp_sessions, mcp_sessions);
}
