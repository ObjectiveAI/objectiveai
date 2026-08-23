//! The client side of a laboratory run: what a client sends.
//!
//! [`request`] opens the scope. [`channel_request`] reaches into the
//! container once it runs. [`channel_response`] answers the channels
//! the provider opens — which, for a caller-served image, is the whole
//! of the image transfer.

pub mod channel_request;
pub mod channel_response;
pub mod request;

// The executor is written and does not compile: it sends the tunneled
// MCP request that this endpoint no longer has, and reads an answer
// split into a head and a body. The five typed exchanges replaced both
// and nothing has been rewired to them yet.
//
// Left whole rather than gutted. What replaces it is a rewrite against
// the new shape, and this is the account of what the endpoint does.
//
// One line restores it.
// pub mod execute;
