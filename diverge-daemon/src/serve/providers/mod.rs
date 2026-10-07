//! The ten requests over providers, served.
//!
//! [`outgoing`] has one handler per request over the providers the
//! daemon dials; [`incoming`] one per request over the credentials of
//! those that dial in. Each the shape [`serve`](super) states, with
//! `connected` taken from what the daemon holds now.

pub mod incoming;
pub mod outgoing;
