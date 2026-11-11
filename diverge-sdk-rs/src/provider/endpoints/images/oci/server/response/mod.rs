//! What a provider sends on an oci scope's main stream: one frame,
//! first — held, not found, unsupported, or the provider's error —
//! see [`Frame`].
//!
//! Nothing is aliased here, because nothing here is somebody else's
//! frame.

mod frame;

pub use frame::*;
