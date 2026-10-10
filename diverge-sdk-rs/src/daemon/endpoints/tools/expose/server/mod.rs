//! The server side of an expose: what the daemon sends.
//!
//! [`response`] is the whole of it: the exposure, not found, forbidden,
//! or the error. The daemon opens no channel of its own on an expose;
//! the one channel on it is the client's cancel.

pub mod response;
