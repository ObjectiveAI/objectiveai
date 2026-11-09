//! List request data.
//!
//! What a caller hands the daemon to list its connected tools: the
//! filter and
//! the count, every one optional, in one [`Frame`]: the [`Filter`] and
//! the count. There is nothing to establish and nothing to resume.

mod filter;
mod frame;

pub use filter::*;
pub use frame::*;
