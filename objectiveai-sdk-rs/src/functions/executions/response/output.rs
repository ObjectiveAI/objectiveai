use crate::functions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wrapper for function execution output, distinguishing between
/// a null output value and a missing output.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "functions.executions.response.Output")]
pub struct Output {
    pub output: functions::expression::TaskOutputOwned,
}

impl Output {
    pub fn unwrap(self) -> functions::expression::TaskOutputOwned {
        self.output
    }
}
