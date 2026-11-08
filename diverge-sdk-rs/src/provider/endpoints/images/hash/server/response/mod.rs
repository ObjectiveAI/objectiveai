//! What a provider sends back on an image hash.
//!
//! One frame, once: the proof, or the word that the caller owns no
//! such image — see [`Frame`].

mod frame;

pub use frame::*;
