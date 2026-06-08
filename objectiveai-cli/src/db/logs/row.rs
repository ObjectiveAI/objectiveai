//! Re-export of the SDK's [`LogTable`] / [`LogValue`] types so callers
//! inside `crate::db::logs` can refer to them without reaching across
//! the crate boundary every time.

pub use objectiveai_sdk::logs::{LogRowIter, LogTable, LogValue};
