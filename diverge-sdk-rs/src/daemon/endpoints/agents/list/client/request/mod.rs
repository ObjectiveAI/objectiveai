//! List request data.
//!
//! What a caller hands the daemon to list its agents: the filter,
//! the program and the count, every one optional: the [`Filter`], which the daemon's own tools reach by
//! too, and the count. There is nothing
//! to establish and nothing to resume, which is why this is one type
//! and not a module of them.

mod filter;
mod frame;

pub use filter::*;
pub use frame::*;
