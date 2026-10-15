//! The server side of a list: what the daemon sends.
//!
//! [`response`] is the whole of it: a tool added, changed or removed,
//! the word that the list is whole, forbidden, or the error. The
//! daemon opens no channel of its own on a list; the one channel on
//! it is the client's cancel.

pub mod response;
