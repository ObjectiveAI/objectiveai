use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct FittingStats {
    pub loss: rust_decimal::Decimal,
    pub executions: usize,
    pub starts: usize,
    pub rounds: usize,
    pub errors: usize,
}
