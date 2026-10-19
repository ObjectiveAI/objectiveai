//! List request data.
//!
//! What a caller hands the daemon to list volumes: the [`Filter`] and
//! the count, each optional.

mod filter;
mod frame;

pub use filter::*;
pub use frame::*;
