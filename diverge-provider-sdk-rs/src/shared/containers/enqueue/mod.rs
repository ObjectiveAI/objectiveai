//! Adding a message to a running loop's queue.
//!
//! The caller opens a channel with [`request::Request`], a message
//! for the loop in flight, and the provider answers once — with one
//! [`response::Frame`] naming the message's fate — whenever that
//! fate is known: taken into the conversation, withdrawn by a
//! [`dequeue`](crate::shared::containers::dequeue), or outlived by
//! the run. Then the finish.
//!
//! Queued, not injected: the turn in flight always runs to
//! completion, and the agent takes the message at a seam of its own
//! choosing. Nothing about this interrupts anything, ever. With no
//! loop running there is nothing to enter, and the message is
//! [`Missed`](response::Frame::Missed).

pub mod request;
pub mod response;
