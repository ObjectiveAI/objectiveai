//! The client side of an MCP plugin: what a client sends.
//!
//! [`request`] opens the scope. [`channel_request`] calls the plugin
//! once it runs, or stops it. [`channel_response`] answers the channel
//! the provider opens — which, for a caller-served image, is the whole
//! of the image transfer.

pub mod channel_request;
pub mod channel_response;
pub mod request;
