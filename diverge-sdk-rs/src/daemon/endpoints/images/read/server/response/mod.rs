//! What the daemon sends on its read scope's main stream: one frame,
//! first — held, not found, forbidden, or the daemon's error — see
//! [`Frame`].

mod frame;

pub use frame::*;
