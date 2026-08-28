//! The answers a server sends on the channels a client opened.
//!
//! [`enqueue`] answers the client's
//! [`Enqueue`](crate::endpoints::agentic_loop::run::client::channel_request::Frame::Enqueue)
//! with the message's fate; [`dequeue`] answers a
//! [`Dequeue`](crate::endpoints::agentic_loop::run::client::channel_request::Frame::Dequeue)
//! with whether the queue held anything. Each answers once, and then
//! the finish.

pub mod dequeue;
pub mod enqueue;
