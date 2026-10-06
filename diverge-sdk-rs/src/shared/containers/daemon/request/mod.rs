//! The ask: one client frame of the daemon connection.
//!
//! [`Request`] is the frame as it crosses a wire, borrowing its
//! payload from the message it arrived in; [`Owned`] is the same frame
//! held, for the caller's relay to keep and hand on.

mod owned;
mod request;

pub use owned::*;
pub use request::*;
