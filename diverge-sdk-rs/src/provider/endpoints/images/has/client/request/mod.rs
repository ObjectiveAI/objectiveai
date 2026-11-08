//! Image has request data.
//!
//! The image asked about, and nothing else — see [`Frame`]. There is
//! nothing to establish and nothing to resume: the answer follows
//! from the question alone.

mod frame;

pub use frame::*;
