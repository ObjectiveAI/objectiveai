//! The agent container's loop, kept: the agent's own server dialled,
//! the queue held, the secrets fed and collected, the conversation
//! relayed.
//!
//! An agent container's entrypoint is an HTTP server on the loopback,
//! at the port [`diverge_sdk::container_proxy::inside::port()`] names, and the
//! loop's three calls are what this module makes of it: `/run`
//! whenever messages wait and no loop runs; `/enqueue` for each
//! message while one does; `/dequeue` when the server clears the
//! queue. The calls every container answers — `/register`,
//! `/secrets`, `/secrets/collect` — are [`program`](crate::program)'s;
//! what the arguments may be is the image's `/DIVERGE.md`, which
//! [`info`](crate::info) reads from the root and asks of no program.
//! The queue itself is
//! the proxy's, held by one [`driver()`] task that never
//! awaits the agent's server directly, so nothing the server asks of
//! the queue can wait on a call the loop holds open.
//!
//! What the loop says rides the begin scope's main stream, chunk by
//! chunk, verbatim, between the proxy's own word that the loop began
//! and its word that it ended — `Active`, one byte, and `Inactive`,
//! carrying the rotating secrets collected from the program — which
//! the proxy alone writes, since the proxy alone makes the `/run`
//! call whose answer and whose end they are. The driver keeps a gate
//! beside the queue: closed at the begin and after every `Inactive`
//! when the program takes secrets, opened by each feed the server
//! sends on its secrets channel once the program has applied the
//! values, and nothing is launched or delivered while it is closed.
//! The one other thing said on the main stream is the `Error` that
//! ends the run when the rotating secrets could not be collected —
//! naming no value — after which the begin loop finishes the scope.

mod dequeue;
mod driver;
mod enqueue;
mod feed;
mod queue;
mod run;

pub use dequeue::*;
pub use driver::*;
pub use enqueue::*;
pub use feed::*;
pub use queue::*;
