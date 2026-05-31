//! User message types.

use super::rich_content::{RichContent, RichContentExpression};
use crate::functions;
use functions::expression::{
    ExpressionError, FromStarlarkValue, WithExpression,
};
use serde::{Deserialize, Serialize};
use starlark::values::dict::DictRef as StarlarkDictRef;
use starlark::values::{UnpackValue, Value as StarlarkValue};
use schemars::JsonSchema;

/// A user message from the end user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.message.UserMessage")]
pub struct UserMessage {
    /// The message content (supports text, images, audio, video, files).
    pub content: RichContent,
    /// Optional name for the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
}

impl UserMessage {
    pub fn push(&mut self, other: &UserMessage) {
        self.content.push(&other.content);
        if self.name.is_none() {
            self.name.clone_from(&other.name);
        }
    }

    pub fn has_name(&self) -> bool {
        self.name.as_ref().is_some_and(|n| !n.is_empty())
    }

    /// Prepares the message by normalizing content and optional fields.
    pub fn prepare(&mut self) {
        self.content.prepare();
        if self.name.as_ref().is_some_and(String::is_empty) {
            self.name = None;
        }
    }

    /// Extract this message's content into per-leaf log files,
    /// returning a [`super::UserMessageLog`] (with
    /// [`super::RichContentLog`] in place of `content`) plus the
    /// [`crate::filesystem::logs::LogFile`]s the caller writes.
    #[cfg(feature = "filesystem")]
    pub fn extract(
        self,
        route_base: &str,
        id: &str,
        message_index: u64,
    ) -> (super::UserMessageLog, Vec<crate::filesystem::logs::LogFile>) {
        let (content, files) = self.content.extract_media(&format!("{route_base}/messages"), id, message_index);
        (super::UserMessageLog { content, name: self.name }, files)
    }
}

impl FromStarlarkValue for UserMessage {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "UserMessage: expected dict".into(),
            )
        })?;
        let mut content = None;
        let mut name = None;
        for (k, v) in dict.iter() {
            let key = <&str as UnpackValue>::unpack_value(k)
                .map_err(|e| {
                    ExpressionError::StarlarkConversionError(e.to_string())
                })?
                .ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "UserMessage: expected string key".into(),
                    )
                })?;
            match key {
                "content" => {
                    content = Some(RichContent::from_starlark_value(&v)?)
                }
                "name" => name = Option::<String>::from_starlark_value(&v)?,
                _ => {}
            }
        }
        Ok(UserMessage {
            content: content.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "UserMessage: missing content".into(),
                )
            })?,
            name,
        })
    }
}

/// Expression variant of [`UserMessage`] for dynamic content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.message.UserMessageExpression")]
pub struct UserMessageExpression {
    /// The message content expression.
    pub content: functions::expression::WithExpression<RichContentExpression>,
    /// Optional name expression.
    #[serde(default, skip_serializing_if = "functions::expression::WithExpression::is_none")]
    #[schemars(with = "Option<functions::expression::WithExpression<String>>", extend("omitempty" = true))]
    pub name: functions::expression::WithExpression<Option<String>>,
}

impl UserMessageExpression {
    /// Compiles the expression into a concrete [`UserMessage`].
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<UserMessage, functions::expression::ExpressionError> {
        let content = self.content.compile_one(params)?.compile(params)?;
        let name = self.name.compile_one(params)?;
        Ok(UserMessage { content, name })
    }
}

impl FromStarlarkValue for UserMessageExpression {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "UserMessageExpression: expected dict".into(),
            )
        })?;
        let mut content = None;
        let mut name = WithExpression::Value(None);
        for (k, v) in dict.iter() {
            let key = <&str as UnpackValue>::unpack_value(k)
                .map_err(|e| {
                    ExpressionError::StarlarkConversionError(e.to_string())
                })?
                .ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "UserMessageExpression: expected string key".into(),
                    )
                })?;
            match key {
                "content" => {
                    content = Some(WithExpression::Value(
                        RichContentExpression::from_starlark_value(&v)?,
                    ))
                }
                "name" => {
                    name = WithExpression::Value(if v.is_none() {
                        None
                    } else {
                        Some(String::from_starlark_value(&v)?)
                    });
                }
                _ => {}
            }
        }
        Ok(UserMessageExpression {
            content: content.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "UserMessageExpression: missing content".into(),
                )
            })?,
            name,
        })
    }
}
