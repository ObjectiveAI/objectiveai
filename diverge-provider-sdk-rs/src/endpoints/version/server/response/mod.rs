//! What a provider sends back for a version request.
//!
//! One frame, once: the version — see [`Frame`].
//!
//! Nothing is aliased here, because nothing here is somebody else's
//! frame.

mod frame;

pub use frame::*;
