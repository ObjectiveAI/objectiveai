//! Core expression types for JMESPath evaluation.

use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Result of an expression that may produce one or many values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    /// A single value.
    One(T),
    /// Multiple values (from array expressions).
    Many(Vec<T>),
}

/// A JMESPath expression.
///
/// Serializes as `{"$jmespath": "expression_string"}` in JSON.
///
/// # Example
///
/// ```json
/// {"$jmespath": "input.items[0].name"}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expression {
    /// The JMESPath expression string.
    #[serde(rename = "$jmespath")]
    pub jmespath: String,
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
        T: DeserializeOwned,
    {
        let expr = super::JMESPATH_RUNTIME.compile(&self.jmespath)?;
        let value = expr.search(params)?;
        let value = serde_json::to_value(value).unwrap();
        let value: Option<OneOrMany<Option<T>>> = serde_json::from_value(value)
            .map_err(super::ExpressionError::DeserializationError)?;
        Ok(match value {
            Some(OneOrMany::One(Some(v))) => OneOrMany::One(v),
            Some(OneOrMany::One(None)) => OneOrMany::Many(Vec::new()),
            Some(OneOrMany::Many(mut vs)) => {
                vs.retain(|v| v.is_some());
                if vs.len() == 0 {
                    OneOrMany::Many(Vec::new())
                } else if vs.len() == 1 {
                    OneOrMany::One(
                        vs.into_iter().filter_map(|v| v).next().unwrap(),
                    )
                } else {
                    OneOrMany::Many(vs.into_iter().filter_map(|v| v).collect())
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
        T: DeserializeOwned,
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

/// A value that can be either a literal or a JMESPath expression.
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
/// Expression:
/// ```json
/// {"$jmespath": "input.greeting"}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WithExpression<T> {
    /// A JMESPath expression to evaluate.
    Expression(Expression),
    /// A literal value.
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
    T: DeserializeOwned,
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
