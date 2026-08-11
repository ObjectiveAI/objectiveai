//! The agentic loop — a provider driving an agent through its turns.
//!
//! A caller sends a [`client::request`] and receives a stream of
//! [`client::response`] chunks.

pub mod client;
