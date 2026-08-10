//! The agentic loop — a provider driving an agent through its turns.
//!
//! A caller sends a [`request`] and receives a stream of
//! [`response`] chunks.

pub mod request;
pub mod response;
