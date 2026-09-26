//! The program's own server, whichever kind of container: dialled
//! for the two calls both kinds answer.
//!
//! Every container's entrypoint is an HTTP server on the loopback, at
//! the port [`diverge_container_proxy_sdk::port()`] names — an
//! agent's loop, or a tool's MCP server — and beside either the two
//! calls this module makes: `/register` once, when the server begins,
//! with the arguments the container was made with; `/schema` when the
//! server asks what they may be. [`Upstream`] is the client both are
//! made with, and the loop's own calls beside them.

mod register;
mod schema;
mod upstream;

pub use register::*;
pub use schema::*;
pub use upstream::*;
