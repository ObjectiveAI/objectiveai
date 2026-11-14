//! Over images: named, never held.
//!
//! An image is no record of the daemon's: a request names one by its
//! digest, and the daemon reaches it through a provider or a remote
//! daemon that can supply it. So a grant over images names
//! nothing and reaches no narrower than every image: on the wire a
//! bare array of the [`Action`]s held, `{"images":["build"]}`, and
//! holding the action is the whole of it, as it is for the events.
//! [`Permission`] is that array as a type.

mod action;
mod permission;

pub use action::*;
pub use permission::*;
