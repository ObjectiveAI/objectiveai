use crate::agent::completions::message::{
    AssistantMessage, DeveloperMessage, ImageUrl, InputAudio, Message,
    RichContent, RichContentPart, SimpleContent, SystemMessage, UserMessage,
};
use crate::agent::claude_agent_sdk;

#[test]
fn no_system_no_prefix_no_suffix() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        ..Default::default()
    };
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("hello".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::User(UserMessage {
            content: RichContent::Text("hello".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn system_prompt_only() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        system_prompt: Some("you are helpful".to_string()),
        ..Default::default()
    };
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("hi".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("you are helpful".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("hi".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn prefix_content_only() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("context info".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::User(UserMessage {
            content: RichContent::Text("context info".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn suffix_content_only() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        suffix_content: Some(RichContent::Text("please be concise".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("please be concise".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn system_prompt_and_prefix_content() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        system_prompt: Some("system".to_string()),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("system".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn system_prompt_and_suffix_content() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        system_prompt: Some("system".to_string()),
        suffix_content: Some(RichContent::Text("suffix".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("system".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("suffix".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn all_three_system_prefix_suffix() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        system_prompt: Some("system".to_string()),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        suffix_content: Some(RichContent::Text("suffix".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("system".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("suffix".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn prefix_inserted_after_leading_system_messages() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys1".to_string()),
            name: None,
        }),
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys2".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys1".to_string()),
            name: None,
        }),
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys2".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn prefix_inserted_after_leading_developer_messages() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::Developer(DeveloperMessage {
            content: SimpleContent::Text("dev".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::Developer(DeveloperMessage {
            content: SimpleContent::Text("dev".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn prefix_inserted_after_mixed_system_and_developer() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys".to_string()),
            name: None,
        }),
        Message::Developer(DeveloperMessage {
            content: SimpleContent::Text("dev".to_string()),
            name: None,
        }),
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys2".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
        Message::Assistant(AssistantMessage {
            content: Some(RichContent::Text("reply".to_string())),
            name: None,
            refusal: None,
            tool_calls: None,
            reasoning: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys".to_string()),
            name: None,
        }),
        Message::Developer(DeveloperMessage {
            content: SimpleContent::Text("dev".to_string()),
            name: None,
        }),
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys2".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
        Message::Assistant(AssistantMessage {
            content: Some(RichContent::Text("reply".to_string())),
            name: None,
            refusal: None,
            tool_calls: None,
            reasoning: None,
        }),
    ]);
}

#[test]
fn prefix_appended_when_all_messages_are_system() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys1".to_string()),
            name: None,
        }),
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys2".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys1".to_string()),
            name: None,
        }),
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys2".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn prefix_with_no_leading_system_messages() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ]);
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
    assert_eq!(merged, vec![
        Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn system_prompt_prepended_before_existing_system_messages() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        system_prompt: Some("agent system".to_string()),
        ..Default::default()
    };
    let messages = vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("existing system".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("agent system".to_string()),
            name: None,
        }),
        Message::System(SystemMessage {
            content: SimpleContent::Text("existing system".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn system_prompt_with_prefix_and_existing_system_and_developer() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        system_prompt: Some("agent system".to_string()),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("existing sys".to_string()),
            name: None,
        }),
        Message::Developer(DeveloperMessage {
            content: SimpleContent::Text("dev".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("agent system".to_string()),
            name: None,
        }),
        Message::System(SystemMessage {
            content: SimpleContent::Text("existing sys".to_string()),
            name: None,
        }),
        Message::Developer(DeveloperMessage {
            content: SimpleContent::Text("dev".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn prefix_not_inserted_twice() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user1".to_string()),
            name: None,
        }),
        Message::Assistant(AssistantMessage {
            content: Some(RichContent::Text("reply".to_string())),
            name: None,
            refusal: None,
            tool_calls: None,
            reasoning: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user2".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user1".to_string()),
            name: None,
        }),
        Message::Assistant(AssistantMessage {
            content: Some(RichContent::Text("reply".to_string())),
            name: None,
            refusal: None,
            tool_calls: None,
            reasoning: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user2".to_string()),
            name: None,
        }),
    ]);
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
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
        Message::Assistant(AssistantMessage {
            content: Some(RichContent::Text("assistant".to_string())),
            name: None,
            refusal: None,
            tool_calls: None,
            reasoning: None,
        }),
    ];
    let merged = agent.merged_messages(messages.clone());
    assert_eq!(merged, messages);
}

#[test]
fn system_prompt_with_empty_messages() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        system_prompt: Some("system".to_string()),
        ..Default::default()
    };
    let messages = vec![];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("system".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn all_three_with_empty_messages() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        system_prompt: Some("system".to_string()),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        suffix_content: Some(RichContent::Text("suffix".to_string())),
        ..Default::default()
    };
    let messages = vec![];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("system".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("suffix".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn prefix_and_suffix_no_system_prompt() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        suffix_content: Some(RichContent::Text("suffix".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("suffix".to_string()),
            name: None,
        }),
    ]);
}

#[test]
fn all_three_with_multi_turn_conversation() {
    let agent = claude_agent_sdk::AgentBase {
        model: "claude-sonnet-4-20250514".to_string(),
        system_prompt: Some("system".to_string()),
        prefix_content: Some(RichContent::Text("prefix".to_string())),
        suffix_content: Some(RichContent::Text("suffix".to_string())),
        ..Default::default()
    };
    let messages = vec![
        Message::Developer(DeveloperMessage {
            content: SimpleContent::Text("dev".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user1".to_string()),
            name: None,
        }),
        Message::Assistant(AssistantMessage {
            content: Some(RichContent::Text("reply1".to_string())),
            name: None,
            refusal: None,
            tool_calls: None,
            reasoning: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user2".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("system".to_string()),
            name: None,
        }),
        Message::Developer(DeveloperMessage {
            content: SimpleContent::Text("dev".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("prefix".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user1".to_string()),
            name: None,
        }),
        Message::Assistant(AssistantMessage {
            content: Some(RichContent::Text("reply1".to_string())),
            name: None,
            refusal: None,
            tool_calls: None,
            reasoning: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user2".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("suffix".to_string()),
            name: None,
        }),
    ]);
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
            RichContentPart::Text { text: "ctx".to_string() },
            RichContentPart::ImageUrl {
                image_url: ImageUrl { url: "https://x/y.png".to_string(), detail: None },
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
