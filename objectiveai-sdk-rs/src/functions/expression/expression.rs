//! Core expression types for JMESPath and Starlark evaluation.

use super::special::FromSpecial;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Result of an expression that may produce one or many values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.expression.OneOrMany.{T}")]
pub enum OneOrMany<T> {
    /// A single value.
    #[schemars(title = "One")]
    One(T),
    /// Multiple values (from array expressions).
    #[schemars(title = "Many")]
    Many(Vec<T>),
}

/// An expression that can be either JMESPath or Starlark.
///
/// Serializes as `{"$jmespath": "..."}` or `{"$starlark": "..."}` in JSON.
///
/// # Examples
///
/// JMESPath:
/// ```json
/// {"$jmespath": "input.items[0].name"}
/// ```
///
/// Starlark:
/// ```json
/// {"$starlark": "input['items'][0]['name']"}
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "functions.expression.Expression")]
pub enum Expression {
    /// A JMESPath expression.
    #[schemars(title = "JMESPath")]
    #[serde(rename = "$jmespath")]
    JMESPath(String),
    /// A Starlark expression.
    #[schemars(title = "Starlark")]
    #[serde(rename = "$starlark")]
    Starlark(String),
    /// A predefined special expression variant.
    #[schemars(title = "Special")]
    #[serde(rename = "$special")]
    Special(super::Special),
}

impl Expression {
    /// Compiles the expression, allowing array results.
    ///
    /// Returns `OneOrMany::One` for single values or `OneOrMany::Many` for arrays.
    /// Null values are filtered out from array results.
    /// A Single Null value is treated as an empty array.
    pub fn compile_one_or_many<T>(
        &self,
        params: &super::Params,
    ) -> Result<OneOrMany<T>, super::ExpressionError>
    where
        T: DeserializeOwned
            + super::starlark::FromStarlarkValue
            + super::special::FromSpecial,
    {
        match self {
            Expression::JMESPath(jmespath) => {
                let expr = super::JMESPATH_RUNTIME.compile(jmespath)?;
                let value = expr.search(params)?;
                let json = serde_json::to_value(value)?;
                Self::deserialize_result(json)
            }
            Expression::Starlark(starlark) => {
                OneOrMany::<T>::from_starlark(starlark, params)
            }
            Expression::Special(special) => {
                OneOrMany::<T>::from_special(special, params)
            }
        }
    }

    /// Deserialize expression result to the expected type.
    fn deserialize_result<T>(
        value: serde_json::Value,
    ) -> Result<OneOrMany<T>, super::ExpressionError>
    where
        T: DeserializeOwned,
    {
        let value: Option<OneOrMany<Option<T>>> = serde_json::from_value(value)
            .map_err(super::ExpressionError::DeserializationError)?;
        Ok(match value {
            Some(OneOrMany::One(Some(v))) => OneOrMany::One(v),
            Some(OneOrMany::One(None)) => OneOrMany::Many(Vec::new()),
            Some(OneOrMany::Many(mut vs)) => {
                vs.retain(|v| v.is_some());
                if vs.is_empty() {
                    OneOrMany::Many(Vec::new())
                } else if vs.len() == 1 {
                    OneOrMany::One(vs.into_iter().flatten().next().unwrap())
                } else {
                    OneOrMany::Many(vs.into_iter().flatten().collect())
                }
            }
            None => OneOrMany::Many(Vec::new()),
        })
    }

    /// Compiles the expression, expecting exactly one value.
    ///
    /// Accepts a single value or an array with exactly one element.
    /// Returns an error if the expression evaluates to null, an empty array,
    /// or an array with more than one element.
    pub fn compile_one<T>(
        &self,
        params: &super::Params,
    ) -> Result<T, super::ExpressionError>
    where
        T: DeserializeOwned
            + super::starlark::FromStarlarkValue
            + super::special::FromSpecial,
    {
        let result = self.compile_one_or_many(params)?;
        match result {
            OneOrMany::One(value) => Ok(value),
            OneOrMany::Many(_) => {
                Err(super::ExpressionError::ExpectedOneValueFoundMany)
            }
        }
    }
}

/// A value that can be either a literal or an expression.
///
/// This allows Function definitions to mix static values with dynamic
/// expressions. During compilation, expressions are evaluated while
/// literal values pass through unchanged.
///
/// # Example
///
/// Literal value:
/// ```json
/// "hello world"
/// ```
///
/// JMESPath expression:
/// ```json
/// {"$jmespath": "input.greeting"}
/// ```
///
/// Starlark expression:
/// ```json
/// {"$starlark": "input['greeting']"}
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(untagged)]
#[schemars(rename = "functions.expression.WithExpression.{T}")]
pub enum WithExpression<T> {
    /// An expression (JMESPath or Starlark) to evaluate.
    #[schemars(title = "Expression")]
    Expression(Expression),
    /// A literal value.
    #[schemars(title = "Value")]
    Value(T),
}

impl<T> std::default::Default for WithExpression<T>
where
    T: Default,
{
    fn default() -> Self {
        WithExpression::Value(T::default())
    }
}

impl<T> WithExpression<T>
where
    T: DeserializeOwned
        + super::starlark::FromStarlarkValue
        + super::special::FromSpecial,
{
    /// Compiles the value, allowing array results from expressions.
    ///
    /// Literal values always return `OneOrMany::One`. Expressions may return
    /// either one or many values.
    pub fn compile_one_or_many(
        self,
        params: &super::Params,
    ) -> Result<OneOrMany<T>, super::ExpressionError> {
        match self {
            WithExpression::Expression(expr) => {
                expr.compile_one_or_many(params)
            }
            WithExpression::Value(value) => Ok(OneOrMany::One(value)),
        }
    }

    /// Compiles the value, expecting exactly one result.
    ///
    /// Literal values pass through unchanged. Expressions must evaluate to
    /// a single value or an array with exactly one element.
    pub fn compile_one(
        self,
        params: &super::Params,
    ) -> Result<T, super::ExpressionError> {
        match self {
            WithExpression::Expression(expr) => expr.compile_one(params),
            WithExpression::Value(value) => Ok(value),
        }
    }
}

impl<T: super::starlark::FromStarlarkValue> super::starlark::FromStarlarkValue
    for WithExpression<T>
{
    fn from_starlark_value(
        value: &starlark::values::Value,
    ) -> Result<Self, super::ExpressionError> {
        T::from_starlark_value(value).map(WithExpression::Value)
    }
}

impl<T: super::special::FromSpecial> FromSpecial
    for super::expression::WithExpression<T>
{
    fn from_special(
        special: &super::special::Special,
        params: &super::Params,
    ) -> Result<Self, super::ExpressionError> {
        Ok(super::expression::WithExpression::Value(T::from_special(
            special, params,
        )?))
    }
}

impl<T> WithExpression<Option<T>> {
    pub fn is_none(&self) -> bool {
        matches!(self, WithExpression::Value(None))
    }
}

