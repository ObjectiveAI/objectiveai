//! The server side of the agentic loop: what a provider sends.
//!
//! [`channel_request`] is what it opens channels of its own to ask
//! for. [`response`] is its answer to the client, on channel `0`.
//! Both happen inside the scope the client's request opened — a server
//! never initiates one.
//!
//! There is no `request` here and no `channel_response`. Only a client
//! opens a scope, and in a loop only a server opens channels, so there
//! is nothing on this side for either name to hold.

pub mod channel_request;
pub mod response;
