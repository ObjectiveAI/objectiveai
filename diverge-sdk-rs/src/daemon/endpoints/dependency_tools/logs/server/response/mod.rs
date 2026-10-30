//! The logs response: the items, one each, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one
//! [`ItemWrapper`](crate::daemon::tool_log::ItemWrapper), forbidden, or
//! a failure. The entries themselves are the tool log's own, in
//! [`tool_log`](crate::daemon::tool_log).

mod frame;

pub use frame::*;

pub use crate::daemon::tool_log::{Item, ItemWrapper};
