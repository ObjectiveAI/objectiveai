//! The `/agent/schema` path: what the agent value may be.
//!
//! Opened by the server, for an agent container, carrying nothing —
//! the opening is the ask. The container answers with one
//! [`response::Frame`]: the JSON Schema of the `agent` value its
//! image accepts, or an `Error`. Then the close.
//!
//! ```text
//! container → server:   [0][schema JSON] | [1][error JSON]    then the close
//! ```
//!
//! # The proxy forwards
//!
//! The schema is the agent's server's, at
//! [`agent::port()`](super::port): the proxy `GET`s its
//! `/schema` and answers the body it got, verbatim, as the schema. A
//! non-`2xx`, or a server that cannot be dialed, is the `Error`, as
//! [`agent`](super) states — and an image that states no
//! schema answers a non-`2xx` saying so. A schema is a courtesy an
//! image extends, not an obligation, and a server that wants one and
//! gets none knows so.

pub mod response;

#[cfg(feature = "server")]
pub mod execute;
