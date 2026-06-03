use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "functions.profiles.computations.response.FittingStats")]
pub struct FittingStats {
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_rust_decimal)]
    pub loss: rust_decimal::Decimal,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_usize)]
    pub executions: usize,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_usize)]
    pub starts: usize,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_usize)]
    pub rounds: usize,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_usize)]
    pub errors: usize,
}
