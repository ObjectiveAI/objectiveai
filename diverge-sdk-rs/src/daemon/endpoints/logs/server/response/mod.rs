//! The logs response: the entries, one each, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Entry`], forbidden,
//! or a failure. An [`Entry`] is one item of the log, wrapped in its
//! subject's kind.

mod entry;
mod frame;

pub use entry::*;
pub use frame::*;
