//! User message types.

use super::rich_content::{RichContent, RichContentExpression};
use crate::functions;
use functions::expression::{
    ExpressionError, FromStarlarkValue, WithExpression,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use starlark::values::dict::DictRef as StarlarkDictRef;
use starlark::values::{UnpackValue, Value as StarlarkValue};

/// A user message from the end user.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.message.UserMessage")]
pub struct UserMessage {
    /// The message content (supports text, images, audio, video, files).
    pub content: RichContent,
}

impl UserMessage {
    pub fn push(&mut self, other: &UserMessage) {
        self.content.push(&other.content);
    }

    /// Prepares the message by normalizing content.
    pub fn prepare(&mut self) {
        self.content.prepare();
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
                _ => {}
            }
        }
        Ok(UserMessage {
            content: content.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "UserMessage: missing content".into(),
                )
            })?,
        })
    }
}

/// Expression variant of [`UserMessage`] for dynamic content.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.message.UserMessageExpression")]
pub struct UserMessageExpression {
    /// The message content expression.
    pub content: functions::expression::WithExpression<RichContentExpression>,
}

impl UserMessageExpression {
    /// Compiles the expression into a concrete [`UserMessage`].
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<UserMessage, functions::expression::ExpressionError> {
        let content = self.content.compile_one(params)?.compile(params)?;
        Ok(UserMessage { content })
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
                _ => {}
            }
        }
        Ok(UserMessageExpression {
            content: content.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "UserMessageExpression: missing content".into(),
                )
            })?,
        })
    }
}
