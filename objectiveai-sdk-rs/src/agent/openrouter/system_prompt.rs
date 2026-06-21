//! The agent's system prompt: a single role-tagged text entry.
//!
//! A [`SystemPrompt`] is a `role` (`system` or `developer`) plus plain `String`
//! `content`. It serializes to the OpenRouter wire shape
//! (`{"role": "system"|"developer", "content": ...}`) and the OpenRouter client
//! prepends it as the leading message of every request.

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
    fn serializes_to_role_tagged_wire_shape() {
        assert_eq!(
            serde_json::to_value(SystemPrompt {
                role: SystemPromptRole::System,
                content: "hi".to_string(),
            })
            .unwrap(),
            serde_json::json!({"role": "system", "content": "hi"})
        );
        assert_eq!(
            serde_json::to_value(SystemPrompt {
                role: SystemPromptRole::Developer,
                content: "yo".to_string(),
            })
            .unwrap(),
            serde_json::json!({"role": "developer", "content": "yo"})
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
