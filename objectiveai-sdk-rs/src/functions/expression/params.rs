//! Parameters and context for expression evaluation.
//!
//! Provides the context available to expressions (JMESPath or Starlark) during
//! compilation, including the function input, task outputs, and current map element.

use super::{ExpressionError, FromStarlarkValue, ToStarlarkValue};
use objectiveai_sdk_macros::schema_override;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use starlark::values::{
    Heap as StarlarkHeap, UnpackValue, Value as StarlarkValue,
};

/// Context for evaluating expressions (JMESPath or Starlark).
///
/// Contains all data accessible within expressions: `input`, `output`, and `map`.
#[schema_override(RefOwnedEnum)]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Params<'i, 'to> {
    /// Owned version (for deserialization).
    Owned(ParamsOwned),
    /// Borrowed version (for efficient evaluation).
    Ref(ParamsRef<'i, 'to>),
}

impl JsonSchema for Params<'static, 'static> {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        ParamsOwned::schema_name()
    }
    fn json_schema(
        generator: &mut schemars::SchemaGenerator,
    ) -> schemars::Schema {
        ParamsOwned::json_schema(generator)
    }
}

impl<'de> serde::Deserialize<'de> for Params<'static, 'static> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let owned = ParamsOwned::deserialize(deserializer)?;
        Ok(Params::Owned(owned))
    }
}

/// Owned version of expression parameters.
#[schema_override(Owned)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.expression.Params")]
pub struct ParamsOwned {
    /// The function's input data.
    pub input: super::InputValue,
    /// Results from executed tasks. Only populated for task output expressions.
    pub output: Option<TaskOutputOwned>,
    /// Current map index. Only populated for mapped task expressions.
    pub map: Option<u64>,
}

/// Borrowed version of expression parameters.
#[schema_override(Ref)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParamsRef<'i, 'to> {
    /// The function's input data.
    pub input: &'i super::InputValue,
    /// Results from executed tasks. Only populated for task output expressions.
    pub output: Option<TaskOutput<'to>>,
    /// Current map index. Only populated for mapped task expressions.
    pub map: Option<u64>,
}

/// Output from an executed task.
#[schema_override(RefOwnedEnum)]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TaskOutput<'a> {
    /// Owned version.
    Owned(TaskOutputOwned),
    /// Borrowed version.
    Ref(TaskOutputRef<'a>),
}

impl JsonSchema for TaskOutput<'static> {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        TaskOutputOwned::schema_name()
    }
    fn json_schema(
        generator: &mut schemars::SchemaGenerator,
    ) -> schemars::Schema {
        TaskOutputOwned::json_schema(generator)
    }
}

impl<'a> super::ToStarlarkValue for TaskOutput<'a> {
    fn to_starlark_value<'v>(
        &self,
        heap: &'v StarlarkHeap,
    ) -> StarlarkValue<'v> {
        match self {
            TaskOutput::Owned(o) => o.to_starlark_value(heap),
            TaskOutput::Ref(r) => r.to_starlark_value(heap),
        }
    }
}

impl<'de> serde::Deserialize<'de> for TaskOutput<'static> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let owned = TaskOutputOwned::deserialize(deserializer)?;
        Ok(TaskOutput::Owned(owned))
    }
}

/// Owned task output variants.
#[schema_override(Owned)]
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(untagged)]
#[schemars(rename = "functions.expression.TaskOutput")]
pub enum TaskOutputOwned {
    /// A single scalar score.
    #[schemars(title = "Scalar")]
    Scalar(
        #[serde(deserialize_with = "crate::serde_util::decimal")]
        #[schemars(with = "f64")]
        #[arbitrary(with = crate::arbitrary_util::arbitrary_rust_decimal)]
        rust_decimal::Decimal,
    ),
    /// A vector of scores.
    #[schemars(title = "Vector")]
    Vector(
        #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
        #[schemars(with = "Vec<f64>")]
        #[arbitrary(with = crate::arbitrary_util::arbitrary_vec_rust_decimal)]
        Vec<rust_decimal::Decimal>,
    ),
    /// Multiple vectors of scores (from mapped tasks).
    #[schemars(title = "Vectors")]
    Vectors(
        #[serde(deserialize_with = "crate::serde_util::vec_vec_decimal")]
        #[schemars(with = "Vec<Vec<f64>>")]
        #[arbitrary(with = crate::arbitrary_util::arbitrary_vec_vec_rust_decimal)]
        Vec<Vec<rust_decimal::Decimal>>,
    ),
    /// An error occurred during execution.
    #[schemars(title = "Err")]
    Err {
        #[arbitrary(with = crate::arbitrary_util::arbitrary_json_value)]
        error: serde_json::Value,
    },
}

impl ToStarlarkValue for TaskOutputOwned {
    fn to_starlark_value<'v>(
        &self,
        heap: &'v StarlarkHeap,
    ) -> StarlarkValue<'v> {
        match self {
            TaskOutputOwned::Scalar(d) => d.to_starlark_value(heap),
            TaskOutputOwned::Vector(ds) => ds.to_starlark_value(heap),
            TaskOutputOwned::Vectors(vecs) => vecs.to_starlark_value(heap),
            TaskOutputOwned::Err { error } => error.to_starlark_value(heap),
        }
    }
}

impl FromStarlarkValue for TaskOutputOwned {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        use starlark::values::float::UnpackFloat;
        if value.is_none() {
            return Ok(TaskOutputOwned::Err {
                error: serde_json::Value::Null,
            });
        }
        if let Some(list) = starlark::values::list::ListRef::from_value(*value)
        {
            // Check if it's a list of lists (Vectors) or list of numbers (Vector)
            let mut all_numeric = true;
            let mut all_lists = true;
            let mut decimals = Vec::with_capacity(list.len());
            let mut vecs = Vec::with_capacity(list.len());

            for v in list.iter() {
                if let Some(inner_list) =
                    starlark::values::list::ListRef::from_value(v)
                {
                    // Try to parse inner list as numbers
                    let mut inner_decimals =
                        Vec::with_capacity(inner_list.len());
                    let mut inner_all_numeric = true;
                    for iv in inner_list.iter() {
                        if let Ok(Some(i)) = i64::unpack_value(iv) {
                            inner_decimals.push(rust_decimal::Decimal::from(i));
                        } else if let Ok(Some(UnpackFloat(f))) =
                            UnpackFloat::unpack_value(iv)
                        {
                            match rust_decimal::Decimal::try_from(f) {
                                Ok(d) => inner_decimals.push(d),
                                Err(_) => {
                                    inner_all_numeric = false;
                                    break;
                                }
                            }
                        } else {
                            inner_all_numeric = false;
                            break;
                        }
                    }
                    if inner_all_numeric {
                        vecs.push(inner_decimals);
                    } else {
                        all_lists = false;
                    }
                    all_numeric = false;
                } else if let Ok(Some(i)) = i64::unpack_value(v) {
                    decimals.push(rust_decimal::Decimal::from(i));
                    all_lists = false;
                } else if let Ok(Some(UnpackFloat(f))) =
                    UnpackFloat::unpack_value(v)
                {
                    match rust_decimal::Decimal::try_from(f) {
                        Ok(d) => {
                            decimals.push(d);
                            all_lists = false;
                        }
                        Err(_) => {
                            all_numeric = false;
                            all_lists = false;
                            break;
                        }
                    }
                } else {
                    all_numeric = false;
                    all_lists = false;
                    break;
                }
            }
            if all_numeric && !decimals.is_empty() {
                return Ok(TaskOutputOwned::Vector(decimals));
            }
            if all_numeric && decimals.is_empty() && list.len() == 0 {
                return Ok(TaskOutputOwned::Vector(Vec::new()));
            }
            if all_lists && !vecs.is_empty() {
                return Ok(TaskOutputOwned::Vectors(vecs));
            }
            if all_lists && vecs.is_empty() && list.len() == 0 {
                return Ok(TaskOutputOwned::Vectors(Vec::new()));
            }
        }
        if let Ok(Some(i)) = i64::unpack_value(*value) {
            return Ok(TaskOutputOwned::Scalar(rust_decimal::Decimal::from(i)));
        }
        if let Ok(Some(UnpackFloat(f))) = UnpackFloat::unpack_value(*value) {
            if let Ok(d) = rust_decimal::Decimal::try_from(f) {
                return Ok(TaskOutputOwned::Scalar(d));
            }
        }
        let v = serde_json::Value::from_starlark_value(value)?;
        Ok(TaskOutputOwned::Err { error: v })
    }
}

impl super::FromSpecial for TaskOutputOwned {
    fn from_special(
        special: &super::Special,
        params: &super::Params,
    ) -> Result<Self, super::ExpressionError> {
        match special {
            super::Special::Output => {
                let output = params_output(params)?;
                Ok(output.clone())
            }
            super::Special::TaskOutputL1Normalized => {
                let output = params_output(params)?;
                match output {
                    TaskOutputOwned::Scalar(_) => Ok(output.clone()),
                    TaskOutputOwned::Vector(v) => {
                        Ok(TaskOutputOwned::Vector(l1_normalize(v)))
                    }
                    TaskOutputOwned::Vectors(vecs) => {
                        Ok(TaskOutputOwned::Vectors(
                            vecs.iter().map(|v| l1_normalize(v)).collect(),
                        ))
                    }
                    TaskOutputOwned::Err { .. } => Ok(output.clone()),
                }
            }
            super::Special::TaskOutputWeightedSum => {
                let output = params_output(params)?;
                match output {
                    TaskOutputOwned::Vector(scores) => {
                        Ok(TaskOutputOwned::Scalar(weighted_sum(scores)))
                    }
                    TaskOutputOwned::Vectors(vecs) => {
                        Ok(TaskOutputOwned::Vector(
                            vecs.iter()
                                .map(|scores| weighted_sum(scores))
                                .collect(),
                        ))
                    }
                    _ => Err(super::ExpressionError::UnsupportedSpecial),
                }
            }
            _ => Err(super::ExpressionError::UnsupportedSpecial),
        }
    }
}

impl TaskOutputOwned {
    /// Converts the output into an error variant (wrapping the value as JSON).
    pub fn into_err(self) -> Self {
        match self {
            Self::Scalar(scalar) => Self::Err {
                error: serde_json::to_value(scalar).unwrap(),
            },
            Self::Vector(vector) => Self::Err {
                error: serde_json::to_value(vector).unwrap(),
            },
            Self::Vectors(vectors) => Self::Err {
                error: serde_json::to_value(vectors).unwrap(),
            },
            Self::Err { error } => Self::Err { error },
        }
    }
}

/// Borrowed task output variants.
#[schema_override(Ref)]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TaskOutputRef<'a> {
    /// A single scalar score.
    Scalar(&'a rust_decimal::Decimal),
    /// A vector of scores.
    Vector(&'a [rust_decimal::Decimal]),
    /// Multiple vectors of scores (from mapped tasks).
    Vectors(&'a [Vec<rust_decimal::Decimal>]),
    /// An error occurred during execution.
    Err { error: &'a serde_json::Value },
}

impl<'a> ToStarlarkValue for TaskOutputRef<'a> {
    fn to_starlark_value<'v>(
        &self,
        heap: &'v StarlarkHeap,
    ) -> StarlarkValue<'v> {
        match self {
            TaskOutputRef::Scalar(d) => d.to_starlark_value(heap),
            TaskOutputRef::Vector(ds) => ds.to_starlark_value(heap),
            TaskOutputRef::Vectors(vecs) => vecs.to_starlark_value(heap),
            TaskOutputRef::Err { error } => error.to_starlark_value(heap),
        }
    }
}

fn params_output<'a>(
    params: &'a super::Params,
) -> Result<&'a TaskOutputOwned, super::ExpressionError> {
    match params {
        super::Params::Owned(o) => o
            .output
            .as_ref()
            .ok_or(super::ExpressionError::UnsupportedSpecial),
        super::Params::Ref(r) => match &r.output {
            Some(TaskOutput::Owned(o)) => Ok(o),
            Some(TaskOutput::Ref(_)) => {
                // We can't return a reference to TaskOutputRef as TaskOutputOwned,
                // but in practice this path uses Owned. If we hit Ref, it's unsupported.
                Err(super::ExpressionError::UnsupportedSpecial)
            }
            None => Err(super::ExpressionError::UnsupportedSpecial),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_output_deserialize_strict_err_wire_format() {
        // JSON number → Scalar
        let parsed: TaskOutputOwned = serde_json::from_str("94").unwrap();
        assert!(matches!(parsed, TaskOutputOwned::Scalar(_)));

        // JSON array of numbers → Vector
        let parsed: TaskOutputOwned =
            serde_json::from_str("[1, 2, 3]").unwrap();
        assert!(matches!(parsed, TaskOutputOwned::Vector(_)));

        // JSON array of arrays → Vectors
        let parsed: TaskOutputOwned =
            serde_json::from_str("[[1, 2], [3, 4]]").unwrap();
        assert!(matches!(parsed, TaskOutputOwned::Vectors(_)));

        // Bare values that previously fell through to Err must now FAIL,
        // since Err is wire-formatted as `{"error": ...}`.
        assert!(serde_json::from_str::<TaskOutputOwned>("null").is_err());
        assert!(serde_json::from_str::<TaskOutputOwned>("true").is_err());
        assert!(serde_json::from_str::<TaskOutputOwned>(r#""94""#).is_err());

        // `{"error": ...}` is now the canonical Err wire form. The inner value
        // unwraps by exactly one level.
        let parsed: TaskOutputOwned =
            serde_json::from_str(r#"{"error": "something"}"#).unwrap();
        assert!(matches!(
            parsed,
            TaskOutputOwned::Err { error: serde_json::Value::String(ref s) } if s == "something"
        ));

        let parsed: TaskOutputOwned =
            serde_json::from_str(r#"{"error": null}"#).unwrap();
        assert!(matches!(
            parsed,
            TaskOutputOwned::Err {
                error: serde_json::Value::Null
            }
        ));

        // Round-trip: Err { error: String("94") } ↔ {"error":"94"}.
        let original = TaskOutputOwned::Err {
            error: serde_json::Value::String("94".to_string()),
        };
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, r#"{"error":"94"}"#);
        let roundtripped: TaskOutputOwned =
            serde_json::from_str(&json).unwrap();
        assert!(matches!(
            roundtripped,
            TaskOutputOwned::Err { error: serde_json::Value::String(ref s) } if s == "94"
        ));

        // Empty array → Vector (not Vectors, since no inner arrays)
        let parsed: TaskOutputOwned = serde_json::from_str("[]").unwrap();
        assert!(
            matches!(parsed, TaskOutputOwned::Vector(_))
                || matches!(parsed, TaskOutputOwned::Vectors(_))
        );
    }
}

fn l1_normalize(v: &[rust_decimal::Decimal]) -> Vec<rust_decimal::Decimal> {
    if v.is_empty() {
        return Vec::new();
    }
    let sum: rust_decimal::Decimal = v.iter().map(|d| d.abs()).sum();
    if sum.is_zero() {
        let uniform =
            rust_decimal::Decimal::ONE / rust_decimal::Decimal::from(v.len());
        vec![uniform; v.len()]
    } else {
        v.iter().map(|d| d / sum).collect()
    }
}

/// Computes a weighted sum of scores where the first element has weight 0,
/// the last element has weight 1, and intermediate elements are evenly spaced.
fn weighted_sum(scores: &[rust_decimal::Decimal]) -> rust_decimal::Decimal {
    let len = scores.len();
    if len <= 1 {
        return scores.iter().sum();
    }
    let mut ws = rust_decimal::Decimal::ZERO;
    let last = len - 1;
    for (i, score) in scores.iter().enumerate() {
        let weight =
            rust_decimal::Decimal::from(i) / rust_decimal::Decimal::from(last);
        ws += score * weight;
    }
    ws
}
