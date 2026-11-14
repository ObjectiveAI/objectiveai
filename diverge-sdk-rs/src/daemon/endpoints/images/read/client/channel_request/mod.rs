//! The channels a caller opens on the daemon's read scope: the three
//! asks, and the cancel. See [`Frame`].

mod frame;

pub use frame::*;
