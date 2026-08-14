//! The provider asking for the content, and the content arriving.
//!
//! A second channel, opened by the provider once a client has named a
//! destination. [`Request`] is the ask; [`Frame`] is what streams
//! back.

mod frame;
mod request;

pub use frame::*;
pub use request::*;
