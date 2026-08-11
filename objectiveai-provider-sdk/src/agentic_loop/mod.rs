//! The agentic loop — a provider driving an agent through its turns.
//!
//! [`client`] is what a client sends and gets back. [`server`] is what
//! a provider asks for along the way, within the scope the client
//! opened.

pub mod client;
pub mod server;
