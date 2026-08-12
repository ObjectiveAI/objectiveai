//! The image check response.
//!
//! [`Response`] is the answer; [`Available`] and [`Unavailable`] are
//! the two things it can be, each with the discriminator that lets the
//! answer go on the wire untagged.

mod available;
mod response;
mod unavailable;

pub use available::*;
pub use response::*;
pub use unavailable::*;
