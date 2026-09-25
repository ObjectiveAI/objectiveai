//! The server side of a filesystem write: what the daemon sends.
//!
//! [`response`] is what comes back on channel `0`: that the file
//! landed, or why not. [`channel_request`] is the one channel the
//! daemon opens, for the content.
//!
//! There is no `channel_response`, because a client opens no channel
//! on a write for the daemon to answer.

pub mod channel_request;
pub mod response;
