//! The `/agent/register` path: the agent, told to the container once.
//!
//! Opened by the server, for an agent container, before any loop. It
//! sends exactly one message — the [`request::Request`], the agent
//! value the container was made with — and the container answers
//! with one [`response::Frame`]: [`Registered`](response::Frame::Registered),
//! or an `Error`. Then the close.
//!
//! ```text
//! server → container:   [request JSON]                       once
//! container → server:   [0] | [1][error JSON]                 then the close
//! ```
//!
//! # The proxy forwards
//!
//! The registration is the agent's server's, at
//! [`agent::port()`](super::port): the proxy `POST`s the
//! request to its `/register`, verbatim, and answers `Registered` on
//! a `2xx`; a non-`2xx`, or a server that cannot be dialed, is the
//! `Error`, as [`agent`](super) states.
//!
//! # Once, and first
//!
//! The agent is fixed for the container's life. The agent's server
//! refuses a second registration whatever it carries, and refuses a
//! `/run` before the first — both its own non-`2xx`, forwarded — so
//! the server registers exactly once, before the first loop, and
//! never again.

pub mod request;
pub mod response;

#[cfg(feature = "server")]
pub mod execute;
