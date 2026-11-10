//! The agent container's loop, kept: the agent's own server dialled,
//! the queue held, the secrets fed and collected, the conversation
//! relayed.
//!
//! An agent container's entrypoint is an HTTP server on the loopback,
//! at the port [`diverge_sdk::container_proxy::inside::port()`] names, and the
//! loop's three calls are what this module makes of it: `/run`
//! whenever messages wait and no loop runs; `/enqueue` with every
//! message waiting, joined in enqueue order, while one does — the
//! program's one slot, filled; `/dequeue`, naming nothing, to take
//! that slot back when a newer message is to be joined to it or a
//! key in it is withdrawn. The calls every container answers — `/register`,
//! `/secrets`, `/secrets/collect` — are [`program`](crate::program)'s;
//! what the arguments may be is the image's `/DIVERGE.md`, which
//! [`info`](crate::info) reads from the root and asks of no program.
//! The queue itself is
//! the proxy's, held by one [`driver()`] task that never
//! awaits the agent's server directly, so nothing the server asks of
//! the queue can wait on a call the loop holds open. The program
//! holds at most one message — the batch the driver offered — and
//! the driver keeps the order: a batch that comes back goes to the
//! front of the queue, ahead of anything newer, and is offered again
//! with it, joined.
//!
//! What the loop says rides the begin scope's main stream, chunk by
//! chunk, verbatim, between the proxy's own word that the loop began
//! and its word that it ended — `Active`, one byte, and `Inactive`,
//! carrying the rotating secrets collected from the program — which
//! the proxy alone writes, since the proxy alone makes the `/run`
//! call whose answer and whose end they are. The driver keeps a gate
//! beside the queue: shut at the begin and after every `Inactive`,
//! whatever the program declared, and opened by each feed the server
//! sends on its secrets channel once the program has applied the
//! values — an empty one too, which is what a program that takes no
//! secret is fed — save the feed the provider makes at the deploy,
//! before any message can be enqueued, which leaves it shut. Nothing
//! is launched or delivered while it is shut. The gate is the go: the
//! server enqueues the messages it means for one turn and feeds after
//! them, so `/run` is the last call, and takes every one of them,
//! joined.
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
