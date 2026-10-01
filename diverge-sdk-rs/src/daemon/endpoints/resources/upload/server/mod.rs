//! The server side of an upload: what the daemon sends.
//!
//! [`channel_request`] is what the daemon opens on the scope, once
//! per file: the ask for that file's content. [`response`] is the one
//! answer on channel `0`, once every content channel has finished.

pub mod channel_request;
pub mod response;
