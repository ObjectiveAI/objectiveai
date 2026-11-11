//! The server side of a listen: what the daemon sends.
//!
//! [`response`] is the whole of it: a message, the count of those a
//! listener missed, forbidden, or the error. The daemon opens no channel
//! of its own on a listen; the one channel on it is the client's cancel.

pub mod response;
