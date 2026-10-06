//! The list response: the templates, one each, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Listed`],
//! forbidden, or a failure. [`Listed`] is what one is: one template as
//! the daemon holds it — its id, when it was made, who made it — its
//! [`creator`](crate::daemon::creator) — its tags, and the template
//! itself.

mod frame;
mod listed;

pub use frame::*;
pub use listed::*;
