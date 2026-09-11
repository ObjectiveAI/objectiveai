//! The image check response.
//!
//! [`Frame`] is what a response frame holds — an answer, or a failure
//! to reach one. [`Response`] is the answer, and [`Available`] and
//! [`Unavailable`] are the two things it can be, each with the
//! discriminator that lets it serialize untagged.
//!
//! A failure is [`shared::error::Error`](crate::shared::error::Error),
//! which is not the same as [`Unavailable`] and is worth not
//! confusing — see [`Frame`].

mod available;
mod frame;
mod response;
mod unavailable;

pub use available::*;
pub use frame::*;
pub use response::*;
pub use unavailable::*;
