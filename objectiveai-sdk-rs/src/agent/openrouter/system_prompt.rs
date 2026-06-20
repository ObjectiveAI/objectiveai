//! The agent's system prompt: a single role-tagged text entry.
//!
//! A [`SystemPrompt`] is a `role` (`system` or `developer`) plus plain `String`
//! `content`. The OpenRouter client renders it (via [`SystemPrompt::to_message`])
//! as the leading message of every request.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The role of a [`SystemPrompt`].
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "agent.openrouter.SystemPromptRole")]
pub enum SystemPromptRole {
    System,
    Developer,
}

/// An agent's system prompt — a role and its text content.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.openrouter.SystemPrompt")]
pub struct SystemPrompt {
    /// Whether this is a system or developer message.
    pub role: SystemPromptRole,
    /// The prompt's text content.
    pub content: String,
}

impl SystemPrompt {
    /// Render this prompt to a completions [`Message`] — a system or developer
    /// message carrying the content as plain text.
    ///
    /// [`Message`]: crate::agent::completions::message::Message
    pub fn to_message(&self) -> crate::agent::completions::message::Message {
        use crate::agent::completions::message::{
            DeveloperMessage, Message, SimpleContent, SystemMessage,
        };
        let content = SimpleContent::Text(self.content.clone());
        match self.role {
            SystemPromptRole::System => {
                Message::System(SystemMessage { content, name: None })
            }
            SystemPromptRole::Developer => {
                Message::Developer(DeveloperMessage { content, name: None })
            }
        }
    }

    /// Validates the prompt: `content` must be non-empty.
    pub fn validate(&self) -> Result<(), String> {
        if self.content.is_empty() {
            return Err("system_prompt.content must not be empty".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_message_maps_role_to_message_variant() {
        use crate::agent::completions::message::{
            DeveloperMessage, Message, SimpleContent, SystemMessage,
        };
        assert_eq!(
            SystemPrompt {
                role: SystemPromptRole::System,
                content: "hi".to_string(),
            }
            .to_message(),
            Message::System(SystemMessage {
                content: SimpleContent::Text("hi".to_string()),
                name: None,
            })
        );
        assert_eq!(
            SystemPrompt {
                role: SystemPromptRole::Developer,
                content: "yo".to_string(),
            }
            .to_message(),
            Message::Developer(DeveloperMessage {
                content: SimpleContent::Text("yo".to_string()),
                name: None,
            })
        );
    }

    #[test]
    fn validate_rejects_empty_content() {
        assert!(
            SystemPrompt {
                role: SystemPromptRole::System,
                content: String::new(),
            }
            .validate()
            .is_err()
        );
        assert!(
            SystemPrompt {
                role: SystemPromptRole::System,
                content: "non-empty".to_string(),
            }
            .validate()
            .is_ok()
        );
    }
}
