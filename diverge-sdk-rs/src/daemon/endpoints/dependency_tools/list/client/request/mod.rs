//! List request data.
//!
//! What a caller hands the daemon to list its dependency templates:
//! the [`Filter`] and the count, each optional. [`Type`] is the one
//! value the filter narrows by that is the template's `type` as a
//! word of its own.

mod filter;
mod frame;
mod template_type;

pub use filter::*;
pub use frame::*;
pub use template_type::*;
