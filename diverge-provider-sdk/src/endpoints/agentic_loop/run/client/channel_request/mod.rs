//! The channels a client opens on a server, mid-run.
//!
//! For most of this endpoint's life there were none — a loop's client
//! sent its request and answered what the server asked back, and the
//! doc here said so. [`Frame`] is what changed that: a conversation
//! that is RUNNING is a conversation a caller may want to add to, and
//! the addition cannot ride a new run request without ending this
//! one.
//!
//! Two requests, both about the queue: [`enqueue`](Frame::Enqueue)
//! puts a message in, [`dequeue`](Frame::Dequeue) clears whatever
//! has not yet been taken. Each is answered once — by an
//! [`enqueue::Frame`](crate::endpoints::agentic_loop::run::server::channel_response::enqueue::Frame)
//! saying what became of the message, and a
//! [`dequeue::Frame`](crate::endpoints::agentic_loop::run::server::channel_response::dequeue::Frame)
//! saying whether the queue held anything.

mod dequeue;
mod enqueue;
mod frame;

pub use dequeue::*;
pub use enqueue::*;
pub use frame::*;
