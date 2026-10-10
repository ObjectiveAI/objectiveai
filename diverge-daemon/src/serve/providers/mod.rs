//! The twenty-one requests over providers and daemons, served.
//!
//! [`outgoing`] has one handler per request over the providers the
//! daemon dials; [`incoming`] one per request over the credentials of
//! those that dial in; [`daemons`] one per request over the records of
//! other daemons, reached through either. Each the shape
//! [`serve`](super) states, with `connected` taken from what the
//! daemon holds now.

pub mod daemons;
pub mod incoming;
pub mod outgoing;
