//! The `/agent/dequeue` path: the running loop's queue, cleared.
//!
//! Opened by the server, for an agent container, carrying nothing —
//! the opening is the ask, the whole queue being the only thing there
//! is to clear. The container answers with one [`response::Frame`],
//! whether the queue held anything. Then the close. Every message
//! withdrawn is also answered on its own `/agent/enqueue`, as
//! dequeued.
//!
//! ```text
//! container → server:   [0] | [1] | [2][error JSON]           then the close
//! ```
//!
//! # The proxy forwards
//!
//! The queue is the agent's server's, at
//! [`agent::port()`](super::port): the proxy `POST`s `{}` to
//! its `/dequeue` and answers what it got, as the frame. A non-`2xx`,
//! or a server that cannot be dialed, is the frame's `Error`, as
//! [`agent`](super) states.

pub mod response;

mod outcome;

pub use outcome::*;

#[cfg(feature = "server")]
pub mod execute;
