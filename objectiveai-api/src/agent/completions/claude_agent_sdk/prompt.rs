use super::super::ContinuationItem;
use super::content_block_param::ContentBlockParam;
use super::sdk_message::{
    MessageParam, MessageParamContent, MessageParamRole, SDKUserMessage, SDKUserMessageType,
};
use objectiveai_sdk::agent::completions::message::{Message, RichContent, SimpleContent, SimpleContentPart};

#[derive(Debug, Clone, PartialEq)]
pub struct Prompt {
    pub system_prompt: Option<String>,
    pub message: SDKUserMessage,
}

fn simple_content_to_text(content: &SimpleContent) -> String {
    match content {
        SimpleContent::Text(s) => s.clone(),
        SimpleContent::Parts(parts) => parts
            .iter()
            .map(|p| match p {
                SimpleContentPart::Text { text } => text.as_str(),
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
    }
}

fn push_rich_content(
    message_content: &mut MessageParamContent,
    content: &RichContent,
) -> Result<(), super::Error> {
    match content {
        RichContent::Text(text) => {
            message_content.push(ContentBlockParam::Text(
                super::content_block_param::TextBlockParam {
                    text: text.clone(),
                    r#type: super::content_block_param::TextBlockParamType::Text,
                    cache_control: None,
                    citations: None,
                },
            ));
        }
        RichContent::Parts(parts) => {
            for part in parts {
                let block = ContentBlockParam::try_from(part).map_err(|e| {
                    super::Error::InvalidMessages(e)
                })?;
                message_content.push(block);
            }
        }
    }
    Ok(())
}

impl Prompt {
    pub fn new(
        messages: &[Message],
        continuation: Option<&[ContinuationItem<super::State>]>,
        request_continuation: Option<&objectiveai_sdk::agent::claude_agent_sdk::Continuation>,
    ) -> Result<Self, super::Error> {
        let mut system_parts: Vec<String> = Vec::new();
        let mut user_msg: Option<&objectiveai_sdk::agent::completions::message::UserMessage> = None;
        let mut saw_user = false;

        for msg in messages {
            match msg {
                Message::System(sys) if !saw_user => {
                    let text = simple_content_to_text(&sys.content);
                    if !text.is_empty() {
                        system_parts.push(text);
                    }
                }
                Message::Developer(dev) if !saw_user => {
                    let text = simple_content_to_text(&dev.content);
                    if !text.is_empty() {
                        system_parts.push(text);
                    }
                }
                Message::User(u) if !saw_user => {
                    saw_user = true;
                    user_msg = Some(u);
                }
                Message::System(_) | Message::Developer(_) => {
                    return Err(super::Error::InvalidMessages(
                        "system/developer messages must precede the user message".to_string(),
                    ));
                }
                Message::User(_) => {
                    return Err(super::Error::InvalidMessages(
                        "only one user message is allowed".to_string(),
                    ));
                }
                Message::Assistant(_) => {
                    return Err(super::Error::InvalidMessages(
                        "assistant messages are not allowed".to_string(),
                    ));
                }
                Message::Tool(_) => {
                    return Err(super::Error::InvalidMessages(
                        "tool messages are not allowed".to_string(),
                    ));
                }
            }
        }

        let system_prompt = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };

        // Build the SDK user message
        let mut content = MessageParamContent::Blocks(vec![]);

        // Track the name from the original user message for validation
        let mut expected_name: Option<&str> = None;

        if let Some(u) = user_msg {
            if u.has_name() {
                let name = u.name.as_deref().unwrap();
                expected_name = Some(name);
                content.push(ContentBlockParam::Text(
                    super::content_block_param::TextBlockParam {
                        text: format!("[{name}] :"),
                        r#type: super::content_block_param::TextBlockParamType::Text,
                        cache_control: None,
                        citations: None,
                    },
                ));
            }
            push_rich_content(&mut content, &u.content)?;
        }

        // Process continuation
        let session_id = if let Some(continuation) = continuation {
            let last_state_pos = continuation
                .iter()
                .rposition(|item| matches!(item, ContinuationItem::State(_)));

            let start = last_state_pos.unwrap_or(0);
            let mut session_id = String::new();

            for (i, item) in continuation.iter().enumerate() {
                if i < start {
                    continue;
                }
                match item {
                    ContinuationItem::State(state) => {
                        session_id = state.session_id.clone();
                    }
                    ContinuationItem::ToolMessage(_) if i > start || last_state_pos.is_none() => {
                        return Err(super::Error::InvalidContinuation(
                            "tool messages must precede a state item".to_string(),
                        ));
                    }
                    ContinuationItem::ToolMessage(_) => {
                        // Tool message before state — skip (handled by earlier turns)
                    }
                    ContinuationItem::UserMessage(u) => {
                        // Validate name consistency
                        let cont_name = u.name.as_deref().filter(|n| !n.is_empty());
                        if let Some(name) = cont_name {
                            if let Some(expected) = expected_name {
                                if name != expected {
                                    return Err(super::Error::InvalidMessages(format!(
                                        "continuation user message name '{name}' does not match expected '{expected}'"
                                    )));
                                }
                            } else {
                                expected_name = Some(name);
                            }
                        }
                        push_rich_content(&mut content, &u.content)?;
                    }
                }
            }

            session_id
        } else {
            String::new()
        };

        // Fall back to request continuation session_id if no internal session found.
        let session_id = if session_id.is_empty() {
            request_continuation
                .map(|rc| rc.session_id.clone())
                .unwrap_or_default()
        } else {
            session_id
        };

        let message = SDKUserMessage {
            r#type: SDKUserMessageType::User,
            message: MessageParam {
                content,
                role: MessageParamRole::User,
            },
            parent_tool_use_id: None,
            is_synthetic: None,
            tool_use_result: None,
            uuid: None,
            session_id: Some(session_id),
        };

        Ok(Prompt {
            system_prompt,
            message,
        })
    }
}
