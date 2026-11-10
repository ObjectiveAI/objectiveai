//! The agent container's loop, kept: the agent's own server dialled,
//! the queue held, the secrets fed and collected, the conversation
//! relayed.
//!
//! An agent container's entrypoint is an HTTP server on the loopback,
//! at the port [`diverge_sdk::container_proxy::inside::port()`] names, and the
//! loop's two calls are what this module makes of it: `/run`
//! whenever messages wait and no loop runs; `/enqueue` with every
//! message waiting, joined in order, while one does — onto
//! the program's own queue, answered at once. The calls every
//! container answers — `/register`,
//! `/secrets`, `/secrets/collect` — are [`program`](crate::program)'s;
//! what the arguments may be is the image's `/DIVERGE.md`, which
//! [`info`](crate::info) reads from the root and asks of no program.
//! The queue itself is
//! the proxy's, held by one [`driver()`] task that never
//! awaits the agent's server directly, so nothing the server asks of
//! the queue can wait on a call the loop holds open. The server
//! enqueues messages a batch at a time — one enqueue channel, one
//! batch, its messages in order, its fate every message's alike — and
//! a batch is `delivered` once the program has it — the `/run` that
//! carried it answered `2xx`, or the `/enqueue` `204` — and never
//! waits for a turn; nothing withdraws one. The driver keeps the
//! order, which is the order the batches reached it and each batch's
//! own: a new batch goes to the back of the queue, one call carrying
//! messages is on the wire at a time, and batches that come back —
//! the run closed under them, `409` — go back to the front, ahead of
//! anything that reached the driver after them, and start the next
//! loop, joined. The daemon sends one batch at a time, each after the
//! last was answered, so in practice the queue holds at most one.
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
//! is launched or sent while it is shut. The gate is the go: the
//! server feeds, and once that is answered sends the messages it
//! means for one turn as one batch, so `/run` takes every one of
//! them, joined; a batch that reaches a shut gate waits behind it for
//! the next feed.
//! The one other thing said on the main stream is the `Error` that
//! ends the run when the rotating secrets could not be collected —
//! naming no value — after which the begin loop finishes the scope.

mod driver;
mod enqueue;
mod feed;
mod queue;
mod run;

pub use driver::*;
pub use enqueue::*;
pub use feed::*;
pub use queue::*;
