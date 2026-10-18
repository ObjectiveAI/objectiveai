//! The server side of a write: what the proxy sends.
//!
//! [`response`] is what comes back on channel `0`: that the file
//! landed, or why not. [`channel_request`] is the one channel the
//! proxy opens, for the content.
//!
//! There is no `channel_response`, because the server opens no
//! channel on a write for the proxy to answer.

pub mod channel_request;
pub mod response;
