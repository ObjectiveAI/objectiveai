//! The server side of the agentic loop: what a provider sends.
//!
//! [`request`] is what it opens channels of its own to ask for.
//! [`response`] is its answer to the client, on channel `0`. Both
//! happen inside the scope the client's request opened — a server
//! never initiates one.

pub mod request;
pub mod response;
