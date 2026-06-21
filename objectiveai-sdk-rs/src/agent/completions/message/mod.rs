//! Message types for agent completions.
//!
//! Messages represent the conversation history sent to the model. Each message
//! has a role (user, assistant, or tool) and content.

mod assistant_message;
mod file_content;
mod pipe_ack;
mod rich_content;
mod tool_message;
mod user_message;

pub use assistant_message::*;
pub use file_content::*;
pub use pipe_ack::*;
pub use rich_content::*;
pub use tool_message::*;
pub use user_message::*;

#[cfg(test)]
mod assistant_message_tests;
#[cfg(all(test, feature = "mcp"))]
mod rich_content_tests;

use crate::functions;
use functions::expression::{ExpressionError, FromStarlarkValue};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use starlark::values::dict::DictRef as StarlarkDictRef;
use starlark::values::{UnpackValue, Value as StarlarkValue};

/// Utilities for working with message prompts.
pub mod prompt {
    use super::Message;
    use schemars::JsonSchema;

    /// Returns whether two messages are a chainable pair — i.e. both
    /// user messages (the only mergeable role).
    fn is_chain(a: &Message, b: &Message) -> bool {
        matches!((a, b), (Message::User(_), Message::User(_)))
    }

    /// Pushes `other` into `target` (same-role merge).
    fn push(target: &mut Message, other: &Message) {
        match (target, other) {
            (Message::User(t), Message::User(o)) => t.push(o),
            _ => unreachable!(),
        }
    }

    /// Prepares a list of messages by normalizing each one, then
    /// merging chains of consecutive user messages.
    pub fn prepare(messages: &mut Vec<Message>) {
        messages.iter_mut().for_each(Message::prepare);

        // scan for any chain to avoid allocation if none exist
        let has_chain = messages.windows(2).any(|w| is_chain(&w[0], &w[1]));
        if !has_chain {
            return;
        }

        let mut merged = Vec::with_capacity(messages.len());
        for msg in messages.drain(..) {
            if let Some(last) = merged.last_mut() {
                if is_chain(last, &msg) {
                    push(last, &msg);
                    continue;
                }
            }
            merged.push(msg);
        }
        *messages = merged;

        // re-prepare after merging
        prepare(messages);
    }

    /// Computes a content-addressed ID for a list of messages.
    pub fn id(messages: &[Message]) -> String {
        let mut hasher = twox_hash::XxHash3_128::with_seed(0);
        hasher.write(serde_json::to_string(messages).unwrap().as_bytes());
        format!("{:0>22}", base62::encode(hasher.finish_128()))
    }
}

/// A message in the conversation.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(tag = "role")]
#[schemars(rename = "agent.completions.message.Message")]
pub enum Message {
    /// A user message from the end user.
    #[schemars(title = "User")]
    #[serde(rename = "user")]
    User(UserMessage),
    /// An assistant message (model's previous response).
    #[schemars(title = "Assistant")]
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    /// A tool message containing the result of a tool call.
    #[schemars(title = "Tool")]
    #[serde(rename = "tool")]
    Tool(ToolMessage),
}

impl Message {
    /// Prepares the message for sending by normalizing its content.
    ///
    /// This method consolidates consecutive text parts, removes empty parts,
    /// and normalizes optional fields (setting empty strings to `None`).
    pub fn prepare(&mut self) {
        match self {
            Message::User(msg) => msg.prepare(),
            Message::Assistant(msg) => msg.prepare(),
            Message::Tool(msg) => msg.prepare(),
        }
    }

}

impl FromStarlarkValue for Message {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "Message: expected dict".into(),
            )
        })?;
        // First pass: find the role
        let mut role = None;
        for (k, v) in dict.iter() {
            if let Ok(Some("role")) = <&str as UnpackValue>::unpack_value(k) {
                role = Some(
                    <&str as UnpackValue>::unpack_value(v)
                        .map_err(|e| {
                            ExpressionError::StarlarkConversionError(
                                e.to_string(),
                            )
                        })?
                        .ok_or_else(|| {
                            ExpressionError::StarlarkConversionError(
                                "Message: expected string role".into(),
                            )
                        })?,
                );
                break;
            }
        }
        let role = role.ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "Message: missing role".into(),
            )
        })?;
        match role {
            "user" => {
                UserMessage::from_starlark_value(value).map(Message::User)
            }
            "assistant" => AssistantMessage::from_starlark_value(value)
                .map(Message::Assistant),
            "tool" => {
                ToolMessage::from_starlark_value(value).map(Message::Tool)
            }
            _ => Err(ExpressionError::StarlarkConversionError(format!(
                "Message: unknown role: {}",
                role
            ))),
        }
    }
}

/// A message with expressions for dynamic content.
///
/// This is the expression variant of [`Message`] used in function definitions
/// where message content can be computed from the function input at runtime.
/// Supports both JMESPath and Starlark expressions.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(tag = "role")]
#[schemars(rename = "agent.completions.message.MessageExpression")]
pub enum MessageExpression {
    #[schemars(title = "User")]
    #[serde(rename = "user")]
    User(UserMessageExpression),
    #[schemars(title = "Assistant")]
    #[serde(rename = "assistant")]
    Assistant(AssistantMessageExpression),
    #[schemars(title = "Tool")]
    #[serde(rename = "tool")]
    Tool(ToolMessageExpression),
}

impl MessageExpression {
    /// Compiles the expression into a concrete [`Message`].
    ///
    /// Evaluates all expressions (JMESPath or Starlark) using the provided
    /// parameters and returns the resulting message.
    ///
    /// # Errors
    ///
    /// Returns an error if any expression evaluation fails.
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<Message, functions::expression::ExpressionError> {
        match self {
            MessageExpression::User(msg) => {
                msg.compile(params).map(Message::User)
            }
            MessageExpression::Assistant(msg) => {
                msg.compile(params).map(Message::Assistant)
            }
            MessageExpression::Tool(msg) => {
                msg.compile(params).map(Message::Tool)
            }
        }
    }
}

impl FromStarlarkValue for MessageExpression {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "MessageExpression: expected dict".into(),
            )
        })?;
        // First pass: find the role
        let mut role = None;
        for (k, v) in dict.iter() {
            if let Ok(Some("role")) = <&str as UnpackValue>::unpack_value(k) {
                role = Some(
                    <&str as UnpackValue>::unpack_value(v)
                        .map_err(|e| {
                            ExpressionError::StarlarkConversionError(
                                e.to_string(),
                            )
                        })?
                        .ok_or_else(|| {
                            ExpressionError::StarlarkConversionError(
                                "MessageExpression: expected string role"
                                    .into(),
                            )
                        })?,
                );
                break;
            }
        }
        let role = role.ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "MessageExpression: missing role".into(),
            )
        })?;
        match role {
            "user" => UserMessageExpression::from_starlark_value(value)
                .map(MessageExpression::User),
            "assistant" => {
                AssistantMessageExpression::from_starlark_value(value)
                    .map(MessageExpression::Assistant)
            }
            "tool" => ToolMessageExpression::from_starlark_value(value)
                .map(MessageExpression::Tool),
            _ => Err(ExpressionError::StarlarkConversionError(format!(
                "MessageExpression: unknown role: {}",
                role
            ))),
        }
    }
}

crate::functions::expression::impl_from_special_unsupported!(MessageExpression,);

impl crate::functions::expression::FromSpecial
    for Vec<crate::functions::expression::WithExpression<MessageExpression>>
{
    fn from_special(
        _special: &crate::functions::expression::Special,
        _params: &crate::functions::expression::Params,
    ) -> Result<Self, crate::functions::expression::ExpressionError> {
        Err(crate::functions::expression::ExpressionError::UnsupportedSpecial)
    }
}
