//! Listening: the direct messages sent to the daemon through its
//! connected providers, merged.
//!
//! Each provider answers its own `daemons::listen`: the messages sent
//! to the daemon's identity there. The daemon holds exactly one such
//! listen on every connected provider whose record says `messageable`
//! — opened and ended by [`sync`](crate::providers::sync), as the
//! record says, whether or not any client listens — and takes every
//! message into one ring that every listener reads at its own pace:
//! [`Listening`], fed by [`listen`] and read by
//! [`providers::daemons::listen`](crate::serve::providers::daemons::listen).
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod listening;

pub use listening::*;
