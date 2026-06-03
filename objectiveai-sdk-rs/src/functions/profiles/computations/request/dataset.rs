use crate::functions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub type Dataset = Vec<DatasetItem>;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.profiles.computations.request.DatasetItem")]
pub struct DatasetItem {
    pub input: functions::expression::InputValue,
    pub target: Target,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "functions.profiles.computations.request.Target")]
pub enum Target {
    #[schemars(title = "Scalar")]
    Scalar {
        #[serde(deserialize_with = "crate::serde_util::decimal")]
        #[schemars(with = "f64")]
        value: rust_decimal::Decimal,
    }, // desired scalar output
    #[schemars(title = "Vector")]
    Vector {
        #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
        #[schemars(with = "Vec<f64>")]
        value: Vec<rust_decimal::Decimal>,
    }, // desired vector output
    #[schemars(title = "VectorWinner")]
    VectorWinner { value: usize }, // desired winning index in vector completion
}
