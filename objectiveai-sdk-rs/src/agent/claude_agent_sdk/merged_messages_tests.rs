use crate::agent::claude_agent_sdk;
use crate::agent::completions::message::{
    AssistantMessage, ImageUrl, InputAudio, Message, RichContent,
    RichContentPart, UserMessage,
};

#[test]
fn no_prefix_no_suffix() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        ..Default::default()
    };
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("hello".to_string()),
    })];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![Message::User(UserMessage {
            content: RichContent::Text("hello".to_string()),
        }),]
    );
}

#[test]
fn system_prompt_is_not_rendered_as_a_message() {
    // The agent's system_prompt is consumed by the Claude prompt builder, not
    // emitted into the merged message list.
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        system_prompt: Some("you are helpful".to_string()),
        ..Default::default()
    };
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("hi".to_string()),
    })];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![Message::User(UserMessage {
            content: RichContent::Text("hi".to_string()),
        }),]
    );
}

#[test]
fn prefix_content_only() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("context info".to_string())),
        ..Default::default()
    };
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("user".to_string()),
    })];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::User(UserMessage {
                content: RichContent::Text("context info".to_string()),
            }),
            Message::User(UserMessage {
                content: RichContent::Text("user".to_string()),
            }),
        ]
    );
}

#[test]
fn suffix_content_only() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        suffix_content: Some(RichContent::Text(
            "please be concise".to_string(),
        )),
        ..Default::default()
    };
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("user".to_string()),
    })];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::User(UserMessage {
                content: RichContent::Text("user".to_string()),
            }),
            Message::User(UserMessage {
                content: RichContent::Text("please be concise".to_string()),
            }),
        ]
    );
}

#[test]
fn prefix_and_suffix() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        suffix_content: Some(RichContent::Text("suffix".to_string())),
        ..Default::default()
    };
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("user".to_string()),
    })];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::User(UserMessage {
                content: RichContent::Text("prefix".to_string()),
            }),
            Message::User(UserMessage {
                content: RichContent::Text("user".to_string()),
            }),
            Message::User(UserMessage {
                content: RichContent::Text("suffix".to_string()),
            }),
        ]
    );
}

#[test]
fn prefix_with_empty_messages() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        ..Default::default()
    };
    let messages = vec![];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
        }),]
    );
}

#[test]
fn prefix_and_suffix_with_empty_messages() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        suffix_content: Some(RichContent::Text("suffix".to_string())),
        ..Default::default()
    };
    let messages = vec![];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::User(UserMessage {
                content: RichContent::Text("prefix".to_string()),
            }),
            Message::User(UserMessage {
                content: RichContent::Text("suffix".to_string()),
            }),
        ]
    );
}

#[test]
fn prefix_and_suffix_with_multi_turn_conversation() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        suffix_content: Some(RichContent::Text("suffix".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("user1".to_string()),
        }),
        Message::Assistant(AssistantMessage {
            content: Some(RichContent::Text("reply1".to_string())),
            refusal: None,
            tool_calls: None,
            reasoning: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user2".to_string()),
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::User(UserMessage {
                content: RichContent::Text("prefix".to_string()),
            }),
            Message::User(UserMessage {
                content: RichContent::Text("user1".to_string()),
            }),
            Message::Assistant(AssistantMessage {
                content: Some(RichContent::Text("reply1".to_string())),
                refusal: None,
                tool_calls: None,
                reasoning: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("user2".to_string()),
            }),
            Message::User(UserMessage {
                content: RichContent::Text("suffix".to_string()),
            }),
        ]
    );
}

#[test]
fn everything_empty() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        ..Default::default()
    };
    let messages = vec![];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![]);
}

#[test]
fn all_none_with_multiple_messages() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        ..Default::default()
    };
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
        }),
        Message::Assistant(AssistantMessage {
            content: Some(RichContent::Text("assistant".to_string())),
            refusal: None,
            tool_calls: None,
            reasoning: None,
        }),
    ];
    let merged = agent.merged_messages(messages.clone());
    assert_eq!(merged, messages);
}

// ---------------------------------------------------------------------------
// validate(): prefix_content / suffix_content media-type restrictions.
// ---------------------------------------------------------------------------

#[test]
fn validate_accepts_text_only_prefix() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("ok".to_string())),
        ..Default::default()
    };
    assert!(agent.validate().is_ok());
}

#[test]
fn validate_accepts_text_and_image_parts_in_suffix() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        suffix_content: Some(RichContent::Parts(vec![
            RichContentPart::Text {
                text: "ctx".to_string(),
            },
            RichContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "https://x/y.png".to_string(),
                    detail: None,
                },
            },
        ])),
        ..Default::default()
    };
    assert!(agent.validate().is_ok());
}

#[test]
fn validate_rejects_audio_in_prefix() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Parts(vec![
            RichContentPart::InputAudio {
                input_audio: InputAudio {
                    data: "AAAA".to_string(),
                    format: "wav".to_string(),
                },
            },
        ])),
        ..Default::default()
    };
    let err = agent.validate().unwrap_err();
    assert!(err.contains("prefix_content"));
    assert!(err.contains("input_audio"));
}

#[test]
fn validate_rejects_audio_in_suffix() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        suffix_content: Some(RichContent::Parts(vec![
            RichContentPart::InputAudio {
                input_audio: InputAudio {
                    data: "AAAA".to_string(),
                    format: "wav".to_string(),
                },
            },
        ])),
        ..Default::default()
    };
    let err = agent.validate().unwrap_err();
    assert!(err.contains("suffix_content"));
    assert!(err.contains("input_audio"));
}
