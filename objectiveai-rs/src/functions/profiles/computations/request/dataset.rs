use crate::functions;
use serde::{Deserialize, Serialize};

pub type Dataset = Vec<DatasetItem>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetItem {
    pub input: functions::expression::Input,
    pub target: Target,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Target {
    Scalar { value: rust_decimal::Decimal }, // desired scalar output
    Vector { value: Vec<rust_decimal::Decimal> }, // desired vector output
    VectorWinner { value: usize }, // desired winning index in vector completion
}
