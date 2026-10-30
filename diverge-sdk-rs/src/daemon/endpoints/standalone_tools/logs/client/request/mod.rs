//! Logs request data.
//!
//! What a caller hands the daemon to read a standalone tool's log: the
//! tool, every way of narrowing what comes back, and a cap on how much,
//! each optional. The kind of item is [`ItemType`], the tool log's own.

mod frame;

pub use frame::*;

pub use crate::daemon::tool_log::ItemType;
