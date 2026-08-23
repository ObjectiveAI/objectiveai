//! The server side of an MCP plugin: what a provider sends.
//!
//! [`channel_request`] is what it opens a channel to ask the caller
//! for while pulling the image. [`channel_response`] answers the
//! channel the caller opens to call the plugin. [`response`] is what
//! comes back on channel `0` once it runs.

//!
//! # And, behind the `server` feature, [`handle`]
//!
//! The largest handler: it deploys the container and then serves both
//! ends at once, for as long as the plugin runs.

pub mod channel_request;
pub mod channel_response;
pub mod response;

#[cfg(feature = "server")]
pub mod handle;
