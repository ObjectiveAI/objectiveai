//! The channels a provider opens on a caller during a push.
//!
//! Three: a manifest, a blob, and the proof, each the run's own ask.
//! See [`Frame`].

mod frame;

pub use frame::*;
