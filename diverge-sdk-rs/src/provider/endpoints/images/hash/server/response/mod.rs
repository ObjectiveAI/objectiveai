//! What a provider sends back on an image hash.
//!
//! One frame, once: the proof, or the word that the provider cannot
//! supply the image — see [`Frame`].

mod frame;

pub use frame::*;
