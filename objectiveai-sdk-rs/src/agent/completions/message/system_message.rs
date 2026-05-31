//! System message types.

use super::simple_content::{SimpleContent, SimpleContentExpression};
use crate::functions;
use functions::expression::{
    ExpressionError, FromStarlarkValue, WithExpression,
};
use serde::{Deserialize, Serialize};
use starlark::values::dict::DictRef as StarlarkDictRef;
use starlark::values::{UnpackValue, Value as StarlarkValue};
use schemars::JsonSchema;

/// A system message setting context or instructions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.message.SystemMessage")]
pub struct SystemMessage {
    /// The message content.
    pub content: SimpleContent,
    /// Optional name for the message author.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
}

impl SystemMessage {
    pub fn push(&mut self, other: &SystemMessage) {
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
    /// returning a [`super::SystemMessageLog`] (with
    /// [`super::SimpleContentLog`] in place of `content`) plus the
    /// [`crate::filesystem::logs::LogFile`]s the caller writes.
    #[cfg(feature = "filesystem")]
    pub fn extract(
        self,
        route_base: &str,
        id: &str,
        message_index: u64,
    ) -> (super::SystemMessageLog, Vec<crate::filesystem::logs::LogFile>) {
        let (content, files) = self.content.extract_media(&format!("{route_base}/messages"), id, message_index);
        (super::SystemMessageLog { content, name: self.name }, files)
    }
}

impl FromStarlarkValue for SystemMessage {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "SystemMessage: expected dict".into(),
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
                        "SystemMessage: expected string key".into(),
                    )
                })?;
            match key {
                "content" => {
                    content = Some(SimpleContent::from_starlark_value(&v)?)
                }
                "name" => name = Option::<String>::from_starlark_value(&v)?,
                _ => {}
            }
        }
        Ok(SystemMessage {
            content: content.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "SystemMessage: missing content".into(),
                )
            })?,
            name,
        })
    }
}

/// Expression variant of [`SystemMessage`] for dynamic content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.message.SystemMessageExpression")]
pub struct SystemMessageExpression {
    /// The message content expression.
    pub content: functions::expression::WithExpression<SimpleContentExpression>,
    /// Optional name expression.
    #[serde(default, skip_serializing_if = "functions::expression::WithExpression::is_none")]
    #[schemars(with = "Option<functions::expression::WithExpression<String>>", extend("omitempty" = true))]
    pub name: functions::expression::WithExpression<Option<String>>,
}

impl SystemMessageExpression {
    /// Compiles the expression into a concrete [`SystemMessage`].
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<SystemMessage, functions::expression::ExpressionError> {
        let content = self.content.compile_one(params)?.compile(params)?;
        let name = self.name.compile_one(params)?;
        Ok(SystemMessage { content, name })
    }
}

impl FromStarlarkValue for SystemMessageExpression {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "SystemMessageExpression: expected dict".into(),
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
                        "SystemMessageExpression: expected string key".into(),
                    )
                })?;
            match key {
                "content" => {
                    content = Some(WithExpression::Value(
                        SimpleContentExpression::from_starlark_value(&v)?,
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
        Ok(SystemMessageExpression {
            content: content.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "SystemMessageExpression: missing content".into(),
                )
            })?,
            name,
        })
    }
}
