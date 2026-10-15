//! What the daemon sends: the server frames of the connection, on the
//! provider's half.
//!
//! [`Frame`] is a frame as it crosses a wire, borrowing its payload
//! from the message it arrived in; [`Owned`] is the same frame held,
//! what the caller's [`Daemon`](crate::provider::client::Daemon) hands
//! back one at a time.

mod frame;
mod owned;

pub use frame::*;
pub use owned::*;
