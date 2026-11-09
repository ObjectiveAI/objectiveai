//! The server side of a providers daemons list: what the daemon sends.
//!
//! [`response`] is the whole of it: an identity added, changed or
//! removed, the word that the listing is whole, forbidden, or the error. The
//! daemon opens no channel of its own on a providers daemons list; the one channel on
//! it is the client's cancel.

pub mod response;
