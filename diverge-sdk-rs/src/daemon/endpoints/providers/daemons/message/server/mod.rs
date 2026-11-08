//! The server side of a message: what the daemon sends.
//!
//! [`response`] is the whole of it: the one answer. The daemon opens no
//! channel of its own on a message; the one channel on it is the
//! client's cancel.

pub mod response;
