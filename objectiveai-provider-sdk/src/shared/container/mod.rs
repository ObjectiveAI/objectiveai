//! What can be done to a container once you have one.
//!
//! [`create`](crate::endpoints::containers::create) and
//! [`connect`](crate::endpoints::containers::connect) differ in how
//! they GET a container — one makes it and owns its life, the other
//! joins somebody else's and asks permission. Once attached, they do
//! the same things to it, and those things are here.
//!
//! Unlike [`http`](super::http) and [`filetree`](super::filetree),
//! which are shapes any endpoint could ride, everything in this module
//! is about containers specifically. It is here rather than in either
//! endpoint because BOTH of them need it, not because it is general.

pub mod read;
