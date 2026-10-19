//! The download response: one chunk each, no such volume or path, the
//! volume held, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds. A chunk is the daemon's
//! one [`Chunk`](crate::daemon::download::Chunk), defined beside the
//! other families' downloads and not repeated here.

mod frame;

pub use frame::*;
