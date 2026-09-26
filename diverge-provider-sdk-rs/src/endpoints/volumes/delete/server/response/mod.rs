//! What a provider sends back on a volume deletion.
//!
//! One frame, once, and it says the volume is gone, or that it is
//! mounted and stays — see [`Frame`].

mod frame;

pub use frame::*;
