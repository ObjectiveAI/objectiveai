//! What a provider sends back on an image edit.
//!
//! One frame, once, and it says the description is kept, or that the
//! caller owns no such image — see [`Frame`].

mod frame;

pub use frame::*;
