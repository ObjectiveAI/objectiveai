//! Message request data.
//!
//! What a caller hands the daemon to ask an agent something: the
//! agent, what is asked — a [`Message`] of one of two kinds — and
//! whether to watch.

mod frame;
mod message;

pub use frame::*;
pub use message::*;
