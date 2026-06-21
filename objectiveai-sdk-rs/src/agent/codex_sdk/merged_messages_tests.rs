use crate::agent::codex_sdk;
use crate::agent::completions::message::{
    ImageUrl, InputAudio, Message, RichContent, RichContentPart, UserMessage,
};

#[test]
fn no_prefix_no_suffix() {
    let agent = codex_sdk::AgentBase {
        model: "gpt-5".to_string(),
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
fn prefix_content_only() {
    let agent = codex_sdk::AgentBase {
        model: "gpt-5".to_string(),
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
    let agent = codex_sdk::AgentBase {
        model: "gpt-5".to_string(),
        suffix_content: Some(RichContent::Text("trailing".to_string())),
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
                content: RichContent::Text("trailing".to_string()),
            }),
        ]
    );
}

#[test]
fn prefix_and_suffix() {
    let agent = codex_sdk::AgentBase {
        model: "gpt-5".to_string(),
        prefix_content: Some(RichContent::Text("ctx".to_string())),
        suffix_content: Some(RichContent::Text("post".to_string())),
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
                content: RichContent::Text("ctx".to_string()),
            }),
            Message::User(UserMessage {
                content: RichContent::Text("user".to_string()),
            }),
            Message::User(UserMessage {
                content: RichContent::Text("post".to_string()),
            }),
        ]
    );
}

// ---------------------------------------------------------------------------
// validate(): prefix_content / suffix_content media-type restrictions.
// ---------------------------------------------------------------------------

#[test]
fn validate_accepts_text_only_prefix() {
    let agent = codex_sdk::AgentBase {
        model: "gpt-5".to_string(),
        prefix_content: Some(RichContent::Text("ok".to_string())),
        ..Default::default()
    };
    assert!(agent.validate().is_ok());
}

#[test]
fn validate_accepts_text_and_image_parts_in_suffix() {
    let agent = codex_sdk::AgentBase {
        model: "gpt-5".to_string(),
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
    let agent = codex_sdk::AgentBase {
        model: "gpt-5".to_string(),
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
    let agent = codex_sdk::AgentBase {
        model: "gpt-5".to_string(),
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
