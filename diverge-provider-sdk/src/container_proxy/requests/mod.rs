//! What rides `/requests`: every ask the container makes.
//!
//! One WebSocket, dialed by the server, carrying [`Frame`]s from the
//! container and nothing back: the answer to each comes on its own
//! path, named by the channel the frame carried — see [the
//! module](super).

mod error;
mod frame;
mod request;

pub use error::*;
pub use frame::*;
pub use request::*;
