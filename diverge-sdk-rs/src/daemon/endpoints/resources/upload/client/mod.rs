//! The client side of an upload: what a client sends.
//!
//! [`request`] opens the scope: the kind, and a directory's paths.
//! [`channel_response`] is what the client answers each content
//! channel the daemon opens with: the file's bytes in pieces, then
//! the finish.

pub mod channel_response;
pub mod request;
