//! The server side of an exec: what the daemon sends.
//!
//! [`response`] is the whole of it: the command's stream, or the one
//! answer that says why there is none.

pub mod response;
