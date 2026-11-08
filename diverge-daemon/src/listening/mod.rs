//! Listening: the direct messages sent to the daemon through its
//! connected providers, merged.
//!
//! Each provider answers its own `messages::listen`: the messages sent
//! to the daemon's identity there. The daemon holds exactly one such
//! listen per connected provider while any client listens — opened at
//! the first listener, cancelled after the last — and takes every
//! message into one ring that every listener reads at its own pace:
//! [`Listening`], read by
//! [`providers::daemons::listen`](crate::serve::providers::daemons::listen).
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod listening;

pub use listening::*;
