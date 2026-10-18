//! List request data.
//!
//! What a caller hands the daemon to list its agents: the [`Filter`]
//! and the count, each optional. There is nothing to establish and
//! nothing to resume, which is why this is one type and not a module of
//! them.

mod filter;
mod frame;

pub use filter::*;
pub use frame::*;
