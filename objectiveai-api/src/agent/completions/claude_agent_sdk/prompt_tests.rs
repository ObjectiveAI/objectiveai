use super::content_block_param::*;
use super::prompt::Prompt;
use super::sdk_message::*;
use crate::agent::completions::ContinuationItem;
use objectiveai_sdk::agent::completions::message::*;

fn blocks(blocks: Vec<ContentBlockParam>) -> MessageParamContent {
    MessageParamContent::Blocks(blocks)
}

fn text_block(text: &str) -> ContentBlockParam {
    ContentBlockParam::Text(TextBlockParam {
        text: text.to_string(),
        r#type: TextBlockParamType::Text,
        cache_control: None,
        citations: None,
    })
}

fn expected(system_prompt: Option<&str>, session_id: &str, content: MessageParamContent) -> Prompt {
    Prompt {
        system_prompt: system_prompt.map(|s| s.to_string()),
        message: SDKUserMessage {
            r#type: SDKUserMessageType::User,
            message: MessageParam {
                content,
                role: MessageParamRole::User,
            },
            parent_tool_use_id: None,
            is_synthetic: None,
            tool_use_result: None,
            uuid: None,
            session_id: Some(session_id.to_string()),
        },
    }
}

// ============================================================
// Success tests
// ============================================================

#[test]
fn test_system_prompt_only() {
    let messages: Vec<Message> = vec![];

    assert_eq!(
        Prompt::new(Some("You are helpful"), &messages, None, None).unwrap(),
        expected(Some("You are helpful"), "", blocks(vec![])),
    );
}

#[test]
fn test_empty_system_prompt_is_none() {
    let messages: Vec<Message> = vec![];

    assert_eq!(
        Prompt::new(Some(""), &messages, None, None).unwrap(),
        expected(None, "", blocks(vec![])),
    );
}

#[test]
fn test_system_prompt_and_user_text() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("What is 2+2?".to_string()),
    })];

    assert_eq!(
        Prompt::new(Some("You are helpful"), &messages, None, None).unwrap(),
        expected(
            Some("You are helpful"),
            "",
            blocks(vec![text_block("What is 2+2?")])
        ),
    );
}

#[test]
fn test_user_text_no_system_prompt() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("Hello".to_string()),
    })];

    assert_eq!(
        Prompt::new(None, &messages, None, None).unwrap(),
        expected(None, "", blocks(vec![text_block("Hello")])),
    );
}

#[test]
fn test_user_with_image_url() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Parts(vec![
            RichContentPart::Text {
                text: "Look at this:".to_string(),
            },
            RichContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "https://example.com/img.png".to_string(),
                    detail: None,
                },
            },
        ]),
    })];

    assert_eq!(
        Prompt::new(None, &messages, None, None).unwrap(),
        expected(
            None,
            "",
            blocks(vec![
                text_block("Look at this:"),
                ContentBlockParam::Image(ImageBlockParam {
                    r#type: ImageBlockParamType::Image,
                    source: ImageSource::URL(URLImageSource {
                        r#type: URLImageSourceType::Url,
                        url: "https://example.com/img.png".to_string(),
                    }),
                    cache_control: None,
                }),
            ])
        ),
    );
}

#[test]
fn test_user_with_base64_image() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Parts(vec![RichContentPart::ImageUrl {
            image_url: ImageUrl {
                url: "data:image/png;base64,iVBORw0KGgo".to_string(),
                detail: None,
            },
        }]),
    })];

    assert_eq!(
        Prompt::new(None, &messages, None, None).unwrap(),
        expected(
            None,
            "",
            blocks(vec![ContentBlockParam::Image(ImageBlockParam {
                r#type: ImageBlockParamType::Image,
                source: ImageSource::Base64(Base64ImageSource {
                    r#type: Base64ImageSourceType::Base64,
                    data: "iVBORw0KGgo".to_string(),
                    media_type: Base64ImageSourceMediaType::ImagePng,
                }),
                cache_control: None,
            })])
        ),
    );
}

#[test]
fn test_user_with_file_url() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Parts(vec![RichContentPart::File {
            file: File {
                file_data: None,
                file_id: None,
                filename: Some("doc.pdf".to_string()),
                file_url: Some("https://example.com/doc.pdf".to_string()),
            },
        }]),
    })];

    assert_eq!(
        Prompt::new(None, &messages, None, None).unwrap(),
        expected(
            None,
            "",
            blocks(vec![ContentBlockParam::Document(DocumentBlockParam {
                r#type: DocumentBlockParamType::Document,
                source: DocumentSource::URLPDF(URLPDFSource {
                    r#type: URLPDFSourceType::Url,
                    url: "https://example.com/doc.pdf".to_string(),
                }),
                cache_control: None,
                citations: None,
                context: None,
                title: Some("doc.pdf".to_string()),
            })])
        ),
    );
}

#[test]
fn test_user_with_base64_pdf() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Parts(vec![
            RichContentPart::Text {
                text: "Review this:".to_string(),
            },
            RichContentPart::File {
                file: File {
                    file_data: Some("JVBERi0xLjQ=".to_string()),
                    filename: Some("report.pdf".to_string()),
                    file_url: None,
                    file_id: None,
                },
            },
        ]),
    })];

    assert_eq!(
        Prompt::new(None, &messages, None, None).unwrap(),
        expected(
            None,
            "",
            blocks(vec![
                text_block("Review this:"),
                ContentBlockParam::Document(DocumentBlockParam {
                    r#type: DocumentBlockParamType::Document,
                    source: DocumentSource::Base64PDF(Base64PDFSource {
                        r#type: Base64PDFSourceType::Base64,
                        data: "JVBERi0xLjQ=".to_string(),
                        media_type: Base64PDFSourceMediaType::ApplicationPdf,
                    }),
                    cache_control: None,
                    citations: None,
                    context: None,
                    title: Some("report.pdf".to_string()),
                }),
            ])
        ),
    );
}

#[test]
fn test_continuation_with_state_and_user() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("Original question".to_string()),
    })];

    let continuation = vec![
        ContinuationItem::State(super::State {
            message_count: 1,
            session_id: "sess-abc".to_string(),
        }),
        ContinuationItem::UserMessage(UserMessage {
            content: RichContent::Text("Follow up".to_string()),
        }),
    ];

    assert_eq!(
        Prompt::new(Some("Context"), &messages, Some(&continuation), None).unwrap(),
        expected(
            Some("Context"),
            "sess-abc",
            blocks(vec![
                text_block("Original question"),
                text_block("Follow up"),
            ])
        ),
    );
}

#[test]
fn test_user_with_plain_text_file() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Parts(vec![RichContentPart::File {
            file: File {
                file_data: Some("fn main() {}".to_string()),
                filename: Some("main.rs".to_string()),
                file_url: None,
                file_id: None,
            },
        }]),
    })];

    assert_eq!(
        Prompt::new(None, &messages, None, None).unwrap(),
        expected(
            None,
            "",
            blocks(vec![ContentBlockParam::Document(DocumentBlockParam {
                r#type: DocumentBlockParamType::Document,
                source: DocumentSource::PlainText(PlainTextSource {
                    r#type: PlainTextSourceType::Text,
                    data: "fn main() {}".to_string(),
                    media_type: PlainTextSourceMediaType::TextPlain,
                }),
                cache_control: None,
                citations: None,
                context: None,
                title: Some("main.rs".to_string()),
            })])
        ),
    );
}

#[test]
fn test_empty_messages() {
    let messages: Vec<Message> = vec![];

    assert_eq!(
        Prompt::new(None, &messages, None, None).unwrap(),
        expected(None, "", blocks(vec![])),
    );
}

// ============================================================
// Error tests
// ============================================================

#[test]
fn test_error_assistant_message() {
    let messages = vec![Message::Assistant(AssistantMessage {
        content: Some(RichContent::Text("hi".to_string())),
        tool_calls: None,
        refusal: None,
        reasoning: None,
    })];

    assert_eq!(
        Prompt::new(None, &messages, None, None).unwrap_err(),
        super::Error::InvalidMessages("assistant messages are not allowed".to_string()),
    );
}

#[test]
fn test_error_tool_message() {
    let messages = vec![Message::Tool(ToolMessage {
        content: RichContent::Text("result".to_string()),
        tool_call_id: "tc_1".to_string(),
        metadata: None,
    })];

    assert_eq!(
        Prompt::new(None, &messages, None, None).unwrap_err(),
        super::Error::InvalidMessages("tool messages are not allowed".to_string()),
    );
}

#[test]
fn test_error_two_user_messages() {
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("First".to_string()),
        }),
        Message::User(UserMessage {
            content: RichContent::Text("Second".to_string()),
        }),
    ];

    assert_eq!(
        Prompt::new(None, &messages, None, None).unwrap_err(),
        super::Error::InvalidMessages("only one user message is allowed".to_string()),
    );
}

#[test]
fn test_error_tool_after_state_in_continuation() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("Q".to_string()),
    })];

    let continuation = vec![
        ContinuationItem::State(super::State {
            message_count: 1,
            session_id: "s1".to_string(),
        }),
        ContinuationItem::ToolMessage(ToolMessage {
            content: RichContent::Text("result".to_string()),
            tool_call_id: "tc1".to_string(),
            metadata: None,
        }),
    ];

    assert_eq!(
        Prompt::new(None, &messages, Some(&continuation), None).unwrap_err(),
        super::Error::InvalidContinuation(
            "tool messages must precede a state item".to_string()
        ),
    );
}

#[test]
fn test_error_audio_content() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Parts(vec![RichContentPart::InputAudio {
            input_audio: InputAudio {
                data: "base64data".to_string(),
                format: "mp3".to_string(),
            },
        }]),
    })];

    assert_eq!(
        Prompt::new(None, &messages, None, None).unwrap_err(),
        super::Error::InvalidMessages(
            "unsupported content type: audio (mp3 format, 10 base64 chars)".to_string()
        ),
    );
}

#[test]
fn test_error_video_content() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Parts(vec![RichContentPart::VideoUrl {
            video_url: VideoUrl {
                url: "https://example.com/vid.mp4".to_string(),
            },
        }]),
    })];

    assert_eq!(
        Prompt::new(None, &messages, None, None).unwrap_err(),
        super::Error::InvalidMessages("unsupported content type: video".to_string()),
    );
}

#[test]
fn test_request_continuation_session_id_fallback() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("Hello".to_string()),
    })];

    let rc = objectiveai_sdk::agent::claude_agent_sdk::Continuation {
        upstream: objectiveai_sdk::agent::claude_agent_sdk::Upstream::default(),
        agent_instance_hierarchy: String::new(),
        session_id: "req-sess-123".to_string(),
        mcp_sessions: indexmap::IndexMap::new(),
    };

    // No internal continuation — should fall back to request continuation session_id.
    assert_eq!(
        Prompt::new(None, &messages, None, Some(&rc)).unwrap(),
        expected(None, "req-sess-123", blocks(vec![text_block("Hello")])),
    );
}

#[test]
fn test_internal_session_id_takes_precedence_over_request() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("Hello".to_string()),
    })];

    let continuation = vec![
        ContinuationItem::State(super::State {
            message_count: 1,
            session_id: "internal-sess".to_string(),
        }),
        ContinuationItem::UserMessage(UserMessage {
            content: RichContent::Text("Follow up".to_string()),
        }),
    ];

    let rc = objectiveai_sdk::agent::claude_agent_sdk::Continuation {
        upstream: objectiveai_sdk::agent::claude_agent_sdk::Upstream::default(),
        agent_instance_hierarchy: String::new(),
        session_id: "req-sess-456".to_string(),
        mcp_sessions: indexmap::IndexMap::new(),
    };

    // Internal continuation has session_id — should use it, not request continuation's.
    assert_eq!(
        Prompt::new(None, &messages, Some(&continuation), Some(&rc)).unwrap(),
        expected(None, "internal-sess", blocks(vec![
            text_block("Hello"),
            text_block("Follow up"),
        ])),
    );
}
