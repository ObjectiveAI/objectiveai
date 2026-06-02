//! Tool message types.

use super::rich_content::{RichContent, RichContentExpression};
use crate::functions;
use functions::expression::{
    ExpressionError, FromStarlarkValue, WithExpression,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use starlark::values::dict::DictRef as StarlarkDictRef;
use starlark::values::{UnpackValue, Value as StarlarkValue};

/// Vendor-extension metadata attached to a tool response. The
/// `objectiveai-mcp-proxy` populates known keys (currently
/// `notifications`); the SDK lossy-decodes the MCP `_meta` bag into
/// this typed shape. Unknown keys are dropped.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.message.ToolResponseMetadata")]
pub struct ToolResponseMetadata {
    /// Count of pending notifications the proxy drained and prepended
    /// to the tool response's `content` before returning. Only set
    /// when at least one notification was drained.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub notifications: Option<u64>,
}

/// A tool message containing the result of a tool call.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.message.ToolMessage")]
pub struct ToolMessage {
    /// The content of the tool response.
    pub content: RichContent,
    /// The ID of the tool call this message responds to.
    pub tool_call_id: String,
    /// Optional vendor-extension metadata, populated by
    /// `objectiveai-mcp-proxy` via MCP's `_meta` extension bag.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub metadata: Option<ToolResponseMetadata>,
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
            metadata: None,
        })
    }
}

/// Expression variant of [`ToolMessage`] for dynamic content.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
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
            metadata: None,
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
