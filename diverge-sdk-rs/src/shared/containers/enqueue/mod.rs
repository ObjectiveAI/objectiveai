//! Sending messages to the agent.
//!
//! The caller opens a channel with [`request::Request`], a batch of
//! messages for the agent, in order, and the provider answers once —
//! with one [`response::Frame`] naming the batch's fate — whenever
//! that fate is known: handed to the agent's program, or the error,
//! when the agent refused it or no run could start on it. Then the
//! finish. A run ending does not lose a batch: what it left waiting
//! starts the next. Nothing withdraws a batch once sent.
//!
//! This is the one way into an agent. With no loop running, the
//! batch starts one: the loop's input is its messages, joined in
//! order as one user turn, and its fate is
//! [`Delivered`](response::Frame::Delivered) as the loop takes it.
//! With a loop running, the batch is queued, not injected, and is
//! [`Delivered`](response::Frame::Delivered) as the agent's program
//! accepts it onto its own queue: the turn in flight always runs to
//! completion, and the agent takes the batch at a seam of its own
//! choosing. Nothing about this interrupts anything,
//! ever. What the agent says, either way, rides the run scope's own
//! main stream — see
//! [`agents::run::server::response`](crate::provider::endpoints::containers::agents::run::server::response)
//! — and no channel carries it.

pub mod request;
pub mod response;
