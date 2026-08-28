//! The queue interface every agentic-loop container serves.
//!
//! A running conversation has a queue, and the queue has exactly two
//! verbs: [`enqueue`] puts a message in, [`dequeue`] clears whatever
//! has not yet been taken. This module is the ONE definition of what
//! those verbs look like over a container's HTTP surface — every
//! agentic-loop container, whatever its upstream, serves these same
//! shapes at the same places, which is what lets a provider relay a
//! caller's
//! [`Enqueue`](crate::endpoints::agentic_loop::run::client::channel_request::Frame::Enqueue)
//! or
//! [`Dequeue`](crate::endpoints::agentic_loop::run::client::channel_request::Frame::Dequeue)
//! without knowing which container is behind it.
//!
//! # The routes
//!
//! On the loop port — `8080`, the same one the run request POSTs to:
//!
//! - `POST /enqueue`, body an [`enqueue::Request`], answered with an
//!   [`enqueue::Response`];
//! - `POST /dequeue`, body a [`dequeue::Request`], answered with a
//!   [`dequeue::Response`].
//!
//! # The response is the fate, whenever it comes
//!
//! An enqueue's HTTP response does not arrive when the message is
//! ACCEPTED — it arrives when the message's fate is known: taken
//! into the conversation, withdrawn, or outlived by the run. That
//! can be much later than the ask, and nothing times it out —
//! nothing in this protocol times anything out. The relaying server
//! holds the caller's channel open for exactly as long as the
//! container holds this response open; the two are the same wait,
//! one hop apart.
//!
//! # Same shape, same behavior
//!
//! The shapes come with obligations, and a container that serves
//! them owes all of it: the queue never interrupts the turn in
//! flight; a delivered message is marked in the response stream by a
//! [`user`](crate::endpoints::agentic_loop::run::server::response::UserChunk)
//! chunk carrying the prompt verbatim at the position it landed; a
//! dequeue answers every pending enqueue as dequeued and delivery is
//! never undone; and a message the run outlives is missed, not
//! errored.

pub mod dequeue;
pub mod enqueue;
