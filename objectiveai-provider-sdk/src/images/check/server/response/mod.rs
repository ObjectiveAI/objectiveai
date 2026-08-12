//! The image check response.
//!
//! [`Frame`] is what comes back on channel `0`; [`Available`] and
//! [`Unavailable`] are the two things it can be, each with the
//! discriminator that lets it go on the wire untagged.

mod available;
mod frame;
mod unavailable;

pub use available::*;
pub use frame::*;
pub use unavailable::*;
