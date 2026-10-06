//! The client side of an upload: what a client sends.
//!
//! [`request`] opens the scope: the volume, the destination, and a
//! directory's files. [`channel_response`] is what the client answers
//! each content channel the daemon opens with: the file's bytes in
//! pieces, then the finish.

pub mod channel_response;
pub mod request;
