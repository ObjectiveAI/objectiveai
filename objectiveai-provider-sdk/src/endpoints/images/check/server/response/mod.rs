//! The image check response.
//!
//! [`Frame`] is what a response frame holds; [`Response`] is what goes
//! on the wire inside it, and [`Available`] and [`Unavailable`] are
//! the two things that can be, each with the discriminator that lets
//! it serialize untagged.

mod available;
mod frame;
mod response;
mod unavailable;

pub use available::*;
pub use frame::*;
pub use response::*;
pub use unavailable::*;
