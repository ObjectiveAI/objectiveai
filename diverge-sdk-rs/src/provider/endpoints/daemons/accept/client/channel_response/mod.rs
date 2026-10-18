//! What the accepting daemon answers the provider's half of a
//! connection with: its server frames, one per channel response.
//! See [`Frame`].

mod frame;

pub use frame::*;
