//! The connections response: the connections, one each, forbidden, or a
//! failure. [`Frame`] is what a response frame holds — one
//! [`Connection`](crate::daemon::endpoints::postgres::Connection),
//! forbidden, or a failure; the connection is defined beside the
//! family.

mod frame;

pub use frame::*;
