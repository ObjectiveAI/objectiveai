//! What a provider sends on a pull's main stream, if anything: one of
//! the two refusals — see [`Frame`].
//!
//! Nothing is aliased here, because nothing here is somebody else's
//! frame.

mod frame;

pub use frame::*;
