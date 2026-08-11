//! The server side of the agentic loop: what a provider asks of a
//! client mid-loop.
//!
//! Always scoped. A provider never initiates — it asks within a scope
//! some client request opened, so nothing here reaches a client that
//! did not already invite it.
//!
//! Two kinds, and they are different in kind rather than in subject:
//! [`mcp`] is a call the client performs and answers, [`postgres`] is
//! access the client brokers.

pub mod mcp;
pub mod postgres;
