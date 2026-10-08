//! The server side of a list_for: what the daemon sends.
//!
//! [`response`] is the whole of it: a container added or removed, the
//! word that the listing is whole, no such provider, forbidden, or the
//! error. The daemon opens no channel of its own on a listing; the one
//! channel on it is the client's cancel.

pub mod response;
