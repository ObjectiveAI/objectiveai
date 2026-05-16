//! Simple text content types for system/developer messages.

use crate::functions;
use functions::expression::{
    ExpressionError, FromStarlarkValue, WithExpression,
};
use serde::{Deserialize, Serialize};
use starlark::values::dict::DictRef as StarlarkDictRef;
use starlark::values::{UnpackValue, Value as StarlarkValue};
use schemars::JsonSchema;

/// Simple text content for system/developer messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(untagged)]
#[schemars(rename = "agent.completions.message.SimpleContent")]
pub enum SimpleContent {
    /// Plain text content.
    #[schemars(title = "Text")]
    Text(String),
    /// Multi-part text content.
    #[schemars(title = "Parts")]
    Parts(Vec<SimpleContentPart>),
}

impl SimpleContent {
    pub fn push(&mut self, other: &SimpleContent) {
        match (&mut *self, other) {
            (SimpleContent::Text(self_text), SimpleContent::Text(other_text)) => {
                self_text.push_str(&other_text);
            }
            (SimpleContent::Text(self_text), SimpleContent::Parts(other_parts)) => {
                let mut parts = Vec::with_capacity(1 + other_parts.len());
                parts.push(SimpleContentPart::Text {
                    text: std::mem::take(self_text),
                });
                parts.extend(other_parts.iter().cloned());
                *self = SimpleContent::Parts(parts);
            }
            (SimpleContent::Parts(self_parts), SimpleContent::Text(other_text)) => {
                self_parts.push(SimpleContentPart::Text {
                    text: other_text.clone(),
                });
            }
            (SimpleContent::Parts(self_parts), SimpleContent::Parts(other_parts)) => {
                self_parts.extend(other_parts.iter().cloned());
            }
        }
    }

    /// Prepares the content by consolidating parts into a single text string.
    pub fn prepare(&mut self) {
        match self {
            SimpleContent::Text(_) => {}
            SimpleContent::Parts(parts) => {
                let text_len = parts
                    .iter()
                    .map(|part| match part {
                        SimpleContentPart::Text { text } => text.len(),
                    })
                    .sum();
                let mut text = String::with_capacity(text_len);
                for part in parts {
                    match part {
                        SimpleContentPart::Text { text: part_text } => {
                            text.push_str(&part_text);
                        }
                    }
                }
                *self = SimpleContent::Text(text)
            }
        }
    }
}

impl FromStarlarkValue for SimpleContent {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        if let Ok(Some(s)) = <&str as UnpackValue>::unpack_value(*value) {
            return Ok(SimpleContent::Text(s.to_owned()));
        }
        let parts = Vec::<SimpleContentPart>::from_starlark_value(value)?;
        Ok(SimpleContent::Parts(parts))
    }
}

/// Expression variant of [`SimpleContent`] for dynamic content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(untagged)]
#[schemars(rename = "agent.completions.message.SimpleContentExpression")]
pub enum SimpleContentExpression {
    /// Plain text content.
    #[schemars(title = "Text")]
    Text(String),
    /// Multi-part text content expressions.
    #[schemars(title = "Parts")]
    Parts(
        Vec<functions::expression::WithExpression<SimpleContentPartExpression>>,
    ),
}

impl SimpleContentExpression {
    /// Compiles the expression into a concrete [`SimpleContent`].
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<SimpleContent, functions::expression::ExpressionError> {
        match self {
            SimpleContentExpression::Text(text) => {
                Ok(SimpleContent::Text(text))
            }
            SimpleContentExpression::Parts(parts) => {
                let mut compiled_parts = Vec::with_capacity(parts.len());
                for part in parts {
                    match part.compile_one_or_many(params)? {
                        functions::expression::OneOrMany::One(one_part) => {
                            compiled_parts.push(one_part.compile(params)?);
                        }
                        functions::expression::OneOrMany::Many(many_parts) => {
                            for part in many_parts {
                                compiled_parts.push(part.compile(params)?);
                            }
                        }
                    }
                }
                Ok(SimpleContent::Parts(compiled_parts))
            }
        }
    }
}

impl FromStarlarkValue for SimpleContentExpression {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        if let Ok(Some(s)) = <&str as UnpackValue>::unpack_value(*value) {
            return Ok(SimpleContentExpression::Text(s.to_owned()));
        }
        let parts = Vec::<WithExpression<SimpleContentPartExpression>>::from_starlark_value(value)?;
        Ok(SimpleContentExpression::Parts(parts))
    }
}

/// A part of simple text content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "agent.completions.message.SimpleContentPart")]
pub enum SimpleContentPart {
    /// A text part.
    Text {
        /// The text content.
        text: String,
    },
}

impl FromStarlarkValue for SimpleContentPart {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "SimpleContentPart: expected dict".into(),
            )
        })?;
        for (k, v) in dict.iter() {
            if let Ok(Some("text")) = <&str as UnpackValue>::unpack_value(k) {
                return Ok(SimpleContentPart::Text {
                    text: String::from_starlark_value(&v)?,
                });
            }
        }
        Err(ExpressionError::StarlarkConversionError(
            "SimpleContentPart: missing text".into(),
        ))
    }
}

/// Expression variant of [`SimpleContentPart`] for dynamic content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "agent.completions.message.SimpleContentPartExpression")]
pub enum SimpleContentPartExpression {
    /// A text part expression.
    Text {
        /// The text expression.
        text: functions::expression::WithExpression<String>,
    },
}

impl SimpleContentPartExpression {
    /// Compiles the expression into a concrete [`SimpleContentPart`].
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<SimpleContentPart, functions::expression::ExpressionError> {
        match self {
            SimpleContentPartExpression::Text { text } => {
                let text = text.compile_one(params)?;
                Ok(SimpleContentPart::Text { text })
            }
        }
    }
}

impl FromStarlarkValue for SimpleContentPartExpression {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let part = SimpleContentPart::from_starlark_value(value)?;
        match part {
            SimpleContentPart::Text { text } => {
                Ok(SimpleContentPartExpression::Text {
                    text: WithExpression::Value(text),
                })
            }
        }
    }
}

crate::functions::expression::impl_from_special_unsupported!(
    SimpleContentExpression,
    SimpleContentPartExpression,
);
