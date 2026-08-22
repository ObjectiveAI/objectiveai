//! The server side of an MCP plugin: what a provider sends.
//!
//! [`channel_request`] is what it opens a channel to ask the caller
//! for while pulling the image. [`channel_response`] answers the
//! channel the caller opens to call the plugin. [`response`] is what
//! comes back on channel `0` once it runs.

pub mod channel_request;
pub mod channel_response;
pub mod response;

// The handler is written and does not compile: it was built around
// `Container::connect`, which is gone while the way into a container is
// reconsidered. The file is left as it was rather than gutted, because
// what replaces it will be a rewrite against the new shape and this is
// the account of what the endpoint has to do.
//
// One line restores it.
// pub mod handle;
