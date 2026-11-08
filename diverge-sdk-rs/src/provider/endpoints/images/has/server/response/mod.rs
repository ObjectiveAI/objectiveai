//! What a provider sends back on an image has.
//!
//! One frame, once: held, not found, unsupported, or the provider's
//! error — see [`Frame`].

mod frame;

pub use frame::*;
