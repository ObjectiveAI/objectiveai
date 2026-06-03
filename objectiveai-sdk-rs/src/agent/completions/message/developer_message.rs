//! Developer message types.

use super::simple_content::{SimpleContent, SimpleContentExpression};
use crate::functions;
use functions::expression::{
    ExpressionError, FromStarlarkValue, WithExpression,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use starlark::values::dict::DictRef as StarlarkDictRef;
use starlark::values::{UnpackValue, Value as StarlarkValue};

/// A developer message.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.message.DeveloperMessage")]
pub struct DeveloperMessage {
    /// The message content.
    pub content: SimpleContent,
    /// Optional name for the message author.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
}

impl DeveloperMessage {
    pub fn push(&mut self, other: &DeveloperMessage) {
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

}

impl FromStarlarkValue for DeveloperMessage {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "DeveloperMessage: expected dict".into(),
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
                        "DeveloperMessage: expected string key".into(),
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
        Ok(DeveloperMessage {
            content: content.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "DeveloperMessage: missing content".into(),
                )
            })?,
            name,
        })
    }
}

/// Expression variant of [`DeveloperMessage`] for dynamic content.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.message.DeveloperMessageExpression")]
pub struct DeveloperMessageExpression {
    /// The message content expression.
    pub content: functions::expression::WithExpression<SimpleContentExpression>,
    /// Optional name expression.
    #[serde(
        default,
        skip_serializing_if = "functions::expression::WithExpression::is_none"
    )]
    #[schemars(with = "Option<functions::expression::WithExpression<String>>", extend("omitempty" = true))]
    pub name: functions::expression::WithExpression<Option<String>>,
}

impl DeveloperMessageExpression {
    /// Compiles the expression into a concrete [`DeveloperMessage`].
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<DeveloperMessage, functions::expression::ExpressionError> {
        let content = self.content.compile_one(params)?.compile(params)?;
        let name = self.name.compile_one(params)?;
        Ok(DeveloperMessage { content, name })
    }
}

impl FromStarlarkValue for DeveloperMessageExpression {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "DeveloperMessageExpression: expected dict".into(),
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
                        "DeveloperMessageExpression: expected string key"
                            .into(),
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
        Ok(DeveloperMessageExpression {
            content: content.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "DeveloperMessageExpression: missing content".into(),
                )
            })?,
            name,
        })
    }
}
