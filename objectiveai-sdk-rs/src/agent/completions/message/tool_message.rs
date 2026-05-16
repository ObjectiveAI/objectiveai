//! Tool message types.

use super::rich_content::{RichContent, RichContentExpression};
use crate::functions;
use functions::expression::{
    ExpressionError, FromStarlarkValue, WithExpression,
};
use serde::{Deserialize, Serialize};
use starlark::values::dict::DictRef as StarlarkDictRef;
use starlark::values::{UnpackValue, Value as StarlarkValue};
use schemars::JsonSchema;

/// A tool message containing the result of a tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.message.ToolMessage")]
pub struct ToolMessage {
    /// The content of the tool response.
    pub content: RichContent,
    /// The ID of the tool call this message responds to.
    pub tool_call_id: String,
}

impl ToolMessage {
    /// Prepares the message by normalizing its content.
    pub fn prepare(&mut self) {
        self.content.prepare();
    }
}

impl FromStarlarkValue for ToolMessage {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "ToolMessage: expected dict".into(),
            )
        })?;
        let mut content = None;
        let mut tool_call_id = None;
        for (k, v) in dict.iter() {
            let key = <&str as UnpackValue>::unpack_value(k)
                .map_err(|e| {
                    ExpressionError::StarlarkConversionError(e.to_string())
                })?
                .ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "ToolMessage: expected string key".into(),
                    )
                })?;
            match key {
                "content" => {
                    content = Some(RichContent::from_starlark_value(&v)?)
                }
                "tool_call_id" => {
                    tool_call_id = Some(String::from_starlark_value(&v)?)
                }
                _ => {}
            }
            if content.is_some() && tool_call_id.is_some() {
                break;
            }
        }
        Ok(ToolMessage {
            content: content.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "ToolMessage: missing content".into(),
                )
            })?,
            tool_call_id: tool_call_id.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "ToolMessage: missing tool_call_id".into(),
                )
            })?,
        })
    }
}

/// Expression variant of [`ToolMessage`] for dynamic content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.message.ToolMessageExpression")]
pub struct ToolMessageExpression {
    /// The content expression.
    pub content: functions::expression::WithExpression<RichContentExpression>,
    /// The tool call ID expression.
    pub tool_call_id: functions::expression::WithExpression<String>,
}

impl ToolMessageExpression {
    /// Compiles the expression into a concrete [`ToolMessage`].
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<ToolMessage, functions::expression::ExpressionError> {
        let content = self.content.compile_one(params)?.compile(params)?;
        let tool_call_id = self.tool_call_id.compile_one(params)?;
        Ok(ToolMessage {
            content,
            tool_call_id,
        })
    }
}

impl FromStarlarkValue for ToolMessageExpression {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "ToolMessageExpression: expected dict".into(),
            )
        })?;
        let mut content = None;
        let mut tool_call_id = None;
        for (k, v) in dict.iter() {
            let key = <&str as UnpackValue>::unpack_value(k)
                .map_err(|e| {
                    ExpressionError::StarlarkConversionError(e.to_string())
                })?
                .ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "ToolMessageExpression: expected string key".into(),
                    )
                })?;
            match key {
                "content" => {
                    content = Some(WithExpression::Value(
                        RichContentExpression::from_starlark_value(&v)?,
                    ))
                }
                "tool_call_id" => {
                    tool_call_id = Some(WithExpression::Value(
                        String::from_starlark_value(&v)?,
                    ))
                }
                _ => {}
            }
            if content.is_some() && tool_call_id.is_some() {
                break;
            }
        }
        Ok(ToolMessageExpression {
            content: content.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "ToolMessageExpression: missing content".into(),
                )
            })?,
            tool_call_id: tool_call_id.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "ToolMessageExpression: missing tool_call_id".into(),
                )
            })?,
        })
    }
}
