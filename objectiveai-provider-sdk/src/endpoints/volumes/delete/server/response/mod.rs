//! What a provider sends back on a volume deletion.
//!
//! One frame, once, and it says the volume is gone — see [`Frame`].

mod frame;

pub use frame::*;
