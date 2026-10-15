//! What the program sends: the client frames of the connection, on the
//! caller's half.
//!
//! [`Frame`] is a frame as it crosses a wire, borrowing its payload
//! from the message it arrived in; [`Owned`] is the same frame held,
//! what the caller's [`Daemon`](crate::provider::client::Daemon) is
//! handed one at a time.

mod frame;
mod owned;

pub use frame::*;
pub use owned::*;
