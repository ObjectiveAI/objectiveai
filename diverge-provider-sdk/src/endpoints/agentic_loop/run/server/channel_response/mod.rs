//! The answers a server sends on the channels a client opened.
//!
//! One so far: [`enqueue`] answers the client's
//! [`Enqueue`](crate::endpoints::agentic_loop::run::client::channel_request::Frame::Enqueue)
//! with the message's fate, once, and then the finish.

pub mod enqueue;
