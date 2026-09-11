//! The `/agent/*` paths: calls to the agent's own server, forwarded.
//!
//! An agent container's entrypoint is an HTTP server on the loopback
//! at the SDK's [`agent::port()`](diverge_provider_sdk::container_proxy::agent::port),
//! and each path here is one call to it — `/register`, `/run`,
//! `/schema`, `/enqueue`, `/dequeue` — made when the provider's server opens the
//! path and not before, its answer re-framed as the wire's. The proxy
//! keeps nothing between calls: no schema, no queue, no attachment.
//! It cannot tell an agent container from a tool container, and does
//! not try; the provider's server knows which it made, and opens
//! these paths only on an agent container.

mod dequeue;
mod enqueue;
mod register;
mod run;
mod schema;
mod upstream;

pub use dequeue::*;
pub use enqueue::*;
pub use register::*;
pub use run::*;
pub use schema::*;
pub use upstream::*;
