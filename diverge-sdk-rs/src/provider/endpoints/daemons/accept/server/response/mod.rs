//! What a provider sends back on an accept: the daemon's identity, or
//! a failure. See [`Frame`].

mod accepting;
mod frame;

pub use accepting::*;
pub use frame::*;
