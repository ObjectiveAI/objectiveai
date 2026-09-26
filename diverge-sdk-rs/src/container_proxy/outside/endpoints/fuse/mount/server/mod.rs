//! The server side of a mount: what the proxy sends.
//!
//! [`response`] is what comes back on channel `0`: that the mount is
//! made, or why not. [`channel_request`] is the mount's asks, each on
//! a channel of its own.
//!
//! There is no `channel_response`, because the server opens no
//! channel on a mount for the proxy to answer.

pub mod channel_request;
pub mod response;
