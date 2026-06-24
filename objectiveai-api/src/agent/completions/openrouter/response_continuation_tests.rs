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
        None,
        &[],
        None,
        ""
    );
    assert_eq!(result, objectiveai_sdk::agent::openrouter::Continuation {
        upstream: objectiveai_sdk::agent::openrouter::Upstream::Openrouter,
        agent_instance_hierarchy: String::new(),
        messages: vec![],
    });
}

#[test]
fn test_messages_only() {
    let client = make_client();
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("Hello".into()),
        }),
    ];
    let result = client.response_continuation(
        None,
        &messages,
        None,
        ""
    );
    assert_eq!(result, objectiveai_sdk::agent::openrouter::Continuation {
        upstream: objectiveai_sdk::agent::openrouter::Upstream::Openrouter,
        agent_instance_hierarchy: String::new(),
        messages: vec![
            Message::User(UserMessage {
                content: RichContent::Text("Hello".into()),
            }),
        ],
    });
}

#[test]
fn test_messages_with_continuation() {
    let client = make_client();
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("Hello".into()),
        }),
    ];
    let continuation = vec![
        ContinuationItem::State(AssistantMessage {
            content: Some(RichContent::Text("Hi there".into())),
            refusal: None, tool_calls: None, reasoning: None,
        }),
        ContinuationItem::UserMessage(UserMessage {
            content: RichContent::Text("Follow up".into()),
        }),
    ];
    let result = client.response_continuation(
        None,
        &messages,
        Some(&continuation),
        ""
    );
    assert_eq!(result, objectiveai_sdk::agent::openrouter::Continuation {
        upstream: objectiveai_sdk::agent::openrouter::Upstream::Openrouter,
        agent_instance_hierarchy: String::new(),
        messages: vec![
            Message::User(UserMessage {
                content: RichContent::Text("Hello".into()),
            }),
            Message::Assistant(AssistantMessage {
                content: Some(RichContent::Text("Hi there".into())),
                refusal: None, tool_calls: None, reasoning: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("Follow up".into()),
            }),
        ],
    });
}

#[test]
fn test_request_continuation_messages_come_first() {
    let client = make_client();
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("Current turn".into()),
        }),
    ];
    let rc = objectiveai_sdk::agent::openrouter::Continuation {
        upstream: objectiveai_sdk::agent::openrouter::Upstream::Openrouter,
        agent_instance_hierarchy: String::new(),
        messages: vec![
            Message::User(UserMessage {
                content: RichContent::Text("Previous turn".into()),
            }),
            Message::Assistant(AssistantMessage {
                content: Some(RichContent::Text("Previous response".into())),
                refusal: None, tool_calls: None, reasoning: None,
            }),
        ],
    };
    let result = client.response_continuation(
        Some(&rc),
        &messages,
        None,
        ""
    );
    assert_eq!(result, objectiveai_sdk::agent::openrouter::Continuation {
        upstream: objectiveai_sdk::agent::openrouter::Upstream::Openrouter,
        agent_instance_hierarchy: String::new(),
        messages: vec![
            Message::User(UserMessage {
                content: RichContent::Text("Previous turn".into()),
            }),
            Message::Assistant(AssistantMessage {
                content: Some(RichContent::Text("Previous response".into())),
                refusal: None, tool_calls: None, reasoning: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("Current turn".into()),
            }),
        ],
    });
}
