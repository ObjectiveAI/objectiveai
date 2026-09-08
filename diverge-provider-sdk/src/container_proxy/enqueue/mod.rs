//! The `/agent/enqueue` path: a message for the running loop's queue.
//!
//! Opened by the server, for an agent container. It sends exactly one
//! message — the [`request::Request`], the message's text — and the
//! container answers with one [`response::Frame`], the message's
//! fate, whenever that fate is known: taken into the conversation,
//! withdrawn by a dequeue, or outlived by the run. Then the close.
//!
//! ```text
//! server → container:   [request JSON]                       once
//! container → server:   [0] | [1] | [2] | [3][error JSON]     then the close
//! ```
//!
//! # The proxy forwards
//!
//! The queue is the agent's server's, at
//! [`agent::port()`](super::agent::port): the proxy `POST`s the
//! request to its `/enqueue` and holds this path open for exactly as
//! long as that call is held — the fate can come long after the ask,
//! and nothing times it out — then answers the fate it got, as the
//! frame. A non-`2xx`, or a server that cannot be dialed, is the
//! frame's `Error`, as [`agent`](super::agent) states. The server
//! leaving before the fate abandons the call; the message's fate is
//! then the agent's server's business alone.

pub mod request;
pub mod response;

#[cfg(feature = "server")]
pub mod execute;
