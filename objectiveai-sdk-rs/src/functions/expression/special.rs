//! Special predefined expression variants.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Predefined expression behaviors that require no user-authored code.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "functions.expression.Special")]
pub enum Special {
    /// Returns the params input as-is.
    #[schemars(title = "Input")]
    Input,
    /// Returns the params output as-is.
    #[schemars(title = "Output")]
    Output,
    /// L1-normalizes the output. Scalar/Err pass through.
    /// Vector: L1 normalize. Vectors: L1 normalize each.
    #[schemars(title = "TaskOutputL1Normalized")]
    TaskOutputL1Normalized,
    /// Weighted sum of the output. Vector → Scalar. Vectors → Vector.
    #[schemars(title = "TaskOutputWeightedSum")]
    TaskOutputWeightedSum,
    /// Returns the length of input['items'] as u64
    #[schemars(title = "InputItemsOutputLength")]
    InputItemsOutputLength,
    /// Splits an input containing items and optionally context into multiple inputs
    #[schemars(title = "InputItemsOptionalContextSplit")]
    InputItemsOptionalContextSplit,
    /// Merges multiple inputs containing items and optionally context into a single input
    #[schemars(title = "InputItemsOptionalContextMerge")]
    InputItemsOptionalContextMerge,
}

/// Trait for types that can be produced from a [`Special`] expression variant.
pub trait FromSpecial: Sized {
    fn from_special(
        special: &Special,
        params: &super::Params,
    ) -> Result<Self, super::ExpressionError>;
}

/// Macro for types that never support any Special variant.
macro_rules! impl_from_special_unsupported {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl $crate::functions::expression::FromSpecial for $ty {
                fn from_special(
                    _special: &$crate::functions::expression::Special,
                    _params: &$crate::functions::expression::Params,
                ) -> Result<Self, $crate::functions::expression::ExpressionError> {
                    Err($crate::functions::expression::ExpressionError::UnsupportedSpecial)
                }
            }
        )+
    };
}
pub(crate) use impl_from_special_unsupported;

impl_from_special_unsupported!(bool, i64, String);

impl<K, V, S> FromSpecial for indexmap::IndexMap<K, V, S>
where
    K: Sized,
    V: Sized,
    S: Sized,
{
    fn from_special(
        _special: &Special,
        _params: &super::Params,
    ) -> Result<Self, super::ExpressionError> {
        Err(super::ExpressionError::UnsupportedSpecial)
    }
}

impl FromSpecial for u64 {
    fn from_special(
        special: &Special,
        params: &super::Params,
    ) -> Result<Self, super::ExpressionError> {
        match special {
            Special::InputItemsOutputLength => {
                let input = match params {
                    super::Params::Owned(o) => &o.input,
                    super::Params::Ref(r) => r.input,
                };
                match input {
                    super::InputValue::Object(map) => match map.get("items") {
                        Some(super::InputValue::Array(arr)) => {
                            Ok(arr.len() as u64)
                        }
                        _ => Err(super::ExpressionError::UnsupportedSpecial),
                    },
                    _ => Err(super::ExpressionError::UnsupportedSpecial),
                }
            }
            _ => Err(super::ExpressionError::UnsupportedSpecial),
        }
    }
}

impl<T: FromSpecial> FromSpecial for super::OneOrMany<T> {
    fn from_special(
        special: &Special,
        params: &super::Params,
    ) -> Result<Self, super::ExpressionError> {
        Ok(super::OneOrMany::One(T::from_special(special, params)?))
    }
}

impl<T: FromSpecial> FromSpecial for Option<T> {
    fn from_special(
        special: &Special,
        params: &super::Params,
    ) -> Result<Self, super::ExpressionError> {
        Ok(Some(T::from_special(special, params)?))
    }
}
