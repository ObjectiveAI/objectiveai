//! The one channel the daemon opens on a client during a filesystem
//! write: for the content. See [`Frame`].

mod frame;

pub use frame::*;
