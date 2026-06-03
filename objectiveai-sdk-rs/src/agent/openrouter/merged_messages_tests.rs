use crate::agent::completions::message::{
    AssistantMessage, DeveloperMessage, Message, RichContent, SimpleContent,
    SystemMessage, UserMessage,
};
use crate::agent::openrouter;

#[test]
fn no_prefix_no_suffix_no_post_system() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        ..Default::default()
    };
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("hello".to_string()),
        name: None,
    })];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![Message::User(UserMessage {
            content: RichContent::Text("hello".to_string()),
            name: None,
        }),]
    );
}

#[test]
fn prefix_only() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        prefix_messages: Some(vec![Message::System(SystemMessage {
            content: SimpleContent::Text("you are helpful".to_string()),
            name: None,
        })]),
        ..Default::default()
    };
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("hi".to_string()),
        name: None,
    })];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::System(SystemMessage {
                content: SimpleContent::Text("you are helpful".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("hi".to_string()),
                name: None,
            }),
        ]
    );
}

#[test]
fn suffix_only() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        suffix_messages: Some(vec![Message::User(UserMessage {
            content: RichContent::Text("please be concise".to_string()),
            name: None,
        })]),
        ..Default::default()
    };
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("hi".to_string()),
        name: None,
    })];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::User(UserMessage {
                content: RichContent::Text("hi".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("please be concise".to_string()),
                name: None,
            }),
        ]
    );
}

#[test]
fn prefix_and_suffix() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        prefix_messages: Some(vec![Message::System(SystemMessage {
            content: SimpleContent::Text("system".to_string()),
            name: None,
        })]),
        suffix_messages: Some(vec![Message::User(UserMessage {
            content: RichContent::Text("suffix".to_string()),
            name: None,
        })]),
        ..Default::default()
    };
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("middle".to_string()),
        name: None,
    })];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::System(SystemMessage {
                content: SimpleContent::Text("system".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("middle".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("suffix".to_string()),
                name: None,
            }),
        ]
    );
}

#[test]
fn empty_messages_with_prefix_and_suffix() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        prefix_messages: Some(vec![Message::System(SystemMessage {
            content: SimpleContent::Text("prefix".to_string()),
            name: None,
        })]),
        suffix_messages: Some(vec![Message::User(UserMessage {
            content: RichContent::Text("suffix".to_string()),
            name: None,
        })]),
        ..Default::default()
    };
    let messages = vec![];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::System(SystemMessage {
                content: SimpleContent::Text("prefix".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("suffix".to_string()),
                name: None,
            }),
        ]
    );
}

#[test]
fn post_system_inserted_before_first_non_system_message() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        post_system_prefix_messages: Some(vec![Message::User(UserMessage {
            content: RichContent::Text("injected".to_string()),
            name: None,
        })]),
        ..Default::default()
    };
    let messages = vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("system".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::System(SystemMessage {
                content: SimpleContent::Text("system".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("injected".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("user".to_string()),
                name: None,
            }),
        ]
    );
}

#[test]
fn post_system_inserted_after_multiple_system_and_developer_messages() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        post_system_prefix_messages: Some(vec![Message::User(UserMessage {
            content: RichContent::Text("injected".to_string()),
            name: None,
        })]),
        ..Default::default()
    };
    let messages = vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("sys1".to_string()),
            name: None,
        }),
        Message::Developer(DeveloperMessage {
            content: SimpleContent::Text("dev1".to_string()),
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
    assert_eq!(
        merged,
        vec![
            Message::System(SystemMessage {
                content: SimpleContent::Text("sys1".to_string()),
                name: None,
            }),
            Message::Developer(DeveloperMessage {
                content: SimpleContent::Text("dev1".to_string()),
                name: None,
            }),
            Message::System(SystemMessage {
                content: SimpleContent::Text("sys2".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("injected".to_string()),
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
        ]
    );
}

#[test]
fn post_system_appended_when_all_messages_are_system() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        post_system_prefix_messages: Some(vec![Message::User(UserMessage {
            content: RichContent::Text("injected".to_string()),
            name: None,
        })]),
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
    assert_eq!(
        merged,
        vec![
            Message::System(SystemMessage {
                content: SimpleContent::Text("sys1".to_string()),
                name: None,
            }),
            Message::System(SystemMessage {
                content: SimpleContent::Text("sys2".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("injected".to_string()),
                name: None,
            }),
        ]
    );
}

#[test]
fn post_system_with_no_leading_system_messages() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        post_system_prefix_messages: Some(vec![Message::User(UserMessage {
            content: RichContent::Text("injected".to_string()),
            name: None,
        })]),
        ..Default::default()
    };
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("user".to_string()),
        name: None,
    })];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::User(UserMessage {
                content: RichContent::Text("injected".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("user".to_string()),
                name: None,
            }),
        ]
    );
}

#[test]
fn post_system_with_empty_messages() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        post_system_prefix_messages: Some(vec![Message::User(UserMessage {
            content: RichContent::Text("injected".to_string()),
            name: None,
        })]),
        ..Default::default()
    };
    let messages = vec![];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![Message::User(UserMessage {
            content: RichContent::Text("injected".to_string()),
            name: None,
        }),]
    );
}

#[test]
fn all_three_prefix_post_system_suffix() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        prefix_messages: Some(vec![Message::System(SystemMessage {
            content: SimpleContent::Text("prefix-sys".to_string()),
            name: None,
        })]),
        post_system_prefix_messages: Some(vec![Message::User(UserMessage {
            content: RichContent::Text("post-sys".to_string()),
            name: None,
        })]),
        suffix_messages: Some(vec![Message::User(UserMessage {
            content: RichContent::Text("suffix".to_string()),
            name: None,
        })]),
        ..Default::default()
    };
    let messages = vec![
        Message::System(SystemMessage {
            content: SimpleContent::Text("inline-sys".to_string()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("user".to_string()),
            name: None,
        }),
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::System(SystemMessage {
                content: SimpleContent::Text("prefix-sys".to_string()),
                name: None,
            }),
            Message::System(SystemMessage {
                content: SimpleContent::Text("inline-sys".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("post-sys".to_string()),
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
        ]
    );
}

#[test]
fn multiple_prefix_messages() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        prefix_messages: Some(vec![
            Message::System(SystemMessage {
                content: SimpleContent::Text("prefix1".to_string()),
                name: None,
            }),
            Message::System(SystemMessage {
                content: SimpleContent::Text("prefix2".to_string()),
                name: None,
            }),
        ]),
        ..Default::default()
    };
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("user".to_string()),
        name: None,
    })];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::System(SystemMessage {
                content: SimpleContent::Text("prefix1".to_string()),
                name: None,
            }),
            Message::System(SystemMessage {
                content: SimpleContent::Text("prefix2".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("user".to_string()),
                name: None,
            }),
        ]
    );
}

#[test]
fn multiple_suffix_messages() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        suffix_messages: Some(vec![
            Message::User(UserMessage {
                content: RichContent::Text("suffix1".to_string()),
                name: None,
            }),
            Message::Assistant(AssistantMessage {
                content: Some(RichContent::Text("suffix2".to_string())),
                name: None,
                refusal: None,
                tool_calls: None,
                reasoning: None,
            }),
        ]),
        ..Default::default()
    };
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("user".to_string()),
        name: None,
    })];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::User(UserMessage {
                content: RichContent::Text("user".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("suffix1".to_string()),
                name: None,
            }),
            Message::Assistant(AssistantMessage {
                content: Some(RichContent::Text("suffix2".to_string())),
                name: None,
                refusal: None,
                tool_calls: None,
                reasoning: None,
            }),
        ]
    );
}

#[test]
fn multiple_post_system_messages() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        post_system_prefix_messages: Some(vec![
            Message::User(UserMessage {
                content: RichContent::Text("injected1".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("injected2".to_string()),
                name: None,
            }),
        ]),
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
    ];
    let merged = agent.merged_messages(messages);
    assert_eq!(
        merged,
        vec![
            Message::System(SystemMessage {
                content: SimpleContent::Text("sys".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("injected1".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("injected2".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("user".to_string()),
                name: None,
            }),
        ]
    );
}

#[test]
fn post_system_not_inserted_twice() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        post_system_prefix_messages: Some(vec![Message::User(UserMessage {
            content: RichContent::Text("injected".to_string()),
            name: None,
        })]),
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
    assert_eq!(
        merged,
        vec![
            Message::System(SystemMessage {
                content: SimpleContent::Text("sys".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("injected".to_string()),
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
        ]
    );
}

#[test]
fn developer_messages_treated_like_system_for_post_system() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        post_system_prefix_messages: Some(vec![Message::User(UserMessage {
            content: RichContent::Text("injected".to_string()),
            name: None,
        })]),
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
    assert_eq!(
        merged,
        vec![
            Message::Developer(DeveloperMessage {
                content: SimpleContent::Text("dev".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("injected".to_string()),
                name: None,
            }),
            Message::User(UserMessage {
                content: RichContent::Text("user".to_string()),
                name: None,
            }),
        ]
    );
}

#[test]
fn everything_empty() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
        ..Default::default()
    };
    let messages = vec![];
    let merged = agent.merged_messages(messages);
    assert_eq!(merged, vec![]);
}

#[test]
fn all_none_with_multiple_messages() {
    let agent = openrouter::AgentBase {
        model: "openai/gpt-4o".to_string(),
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
