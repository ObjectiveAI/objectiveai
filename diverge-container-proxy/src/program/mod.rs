//! The program's own server, whichever kind of container: dialled
//! for the calls both kinds answer.
//!
//! Every container's entrypoint is an HTTP server on the loopback, at
//! the port [`diverge_sdk::container_proxy::inside::port()`] names — an
//! agent's loop, or a tool's MCP server — and beside either the calls
//! this module makes: `/register` once, when the server begins, with
//! the arguments the container was made with, answered with the
//! dependencies the program declares and the secrets it takes;
//! `/secrets` with the values of those secrets at every activation,
//! and `/secrets/collect` for the rotating ones at every inactive.
//! [`Upstream`] is the client all of them are made with, and the
//! loop's own calls beside them. What the arguments may be is asked
//! of no program: it is the image's `/DIVERGE.md`, which
//! [`info`](crate::info) reads from the root.

mod register;
mod secrets;
mod upstream;

pub use register::*;
pub use secrets::*;
pub use upstream::*;
