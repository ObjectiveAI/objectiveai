//! The expose response: the exposure, no such tool, forbidden, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one [`Exposed`],
//! forbidden, or a failure. [`Exposed`] is what joins the tool.

mod exposed;
mod frame;

pub use exposed::*;
pub use frame::*;
