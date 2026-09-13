//! The agent container's loop, kept: the agent's own server dialled,
//! the queue held, the conversation relayed.
//!
//! An agent container's entrypoint is an HTTP server on the loopback,
//! at the port [`diverge_container_proxy_sdk::agent`] names, and its
//! five calls are what this module makes of it: `/register` once,
//! when the server begins; `/run` whenever messages wait and no loop
//! runs; `/enqueue` for each message while one does; `/dequeue` when
//! the server clears the queue; `/schema` when it asks. The queue
//! itself is the proxy's, held by one [`driver()`] task that never
//! awaits the agent's server directly, so nothing the server asks of
//! the queue can wait on a call the loop holds open.
//!
//! What the loop says rides the begin scope's main stream, chunk by
//! chunk, verbatim; nothing else is said there, because an error on
//! that stream would end the scope, and the scope is the connection's
//! life.

mod dequeue;
mod driver;
mod enqueue;
mod queue;
mod register;
mod run;
mod schema;
mod upstream;

pub use dequeue::*;
pub use driver::*;
pub use enqueue::*;
pub use queue::*;
pub use register::*;
pub use schema::*;
pub use upstream::*;
