//! The agentic loop — a provider driving an agent through its turns.
//!
//! Split by who SENDS, not by who a thing is about. [`client`] is
//! everything a client puts on the wire: its request, and its answers
//! to whatever the server asked for along the way. [`server`] is
//! everything a provider puts on the wire: the answer to that request,
//! and the requests of its own it makes inside the scope that request
//! opened.

pub mod client;
pub mod server;
