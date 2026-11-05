//! The list response: the daemons, one value each, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Daemon`],
//! forbidden, or a failure. [`Daemon`] is what one is.

mod frame;

pub use frame::*;
pub use crate::daemon::endpoints::daemons::Daemon;
