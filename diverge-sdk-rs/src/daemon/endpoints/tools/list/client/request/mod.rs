//! List request data.
//!
//! What a caller hands the daemon to list its tools: the filter, the
//! program and the count, every one optional, in one [`Frame`]: the
//! [`Filter`] and the count; [`Kind`] is the one value of its own the
//! filter names. There is nothing to establish and nothing to resume.

mod filter;
mod frame;
mod kind;

pub use filter::*;
pub use frame::*;
pub use kind::*;
