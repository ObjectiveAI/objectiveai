//! The server side of an identities login: what the daemon sends.
//!
//! [`response`] is the whole of it: the code to approve, then the
//! outcome; or a refusal. The daemon opens no channel of its own on a
//! login; the one channel on it is the client's cancel.

pub mod response;
