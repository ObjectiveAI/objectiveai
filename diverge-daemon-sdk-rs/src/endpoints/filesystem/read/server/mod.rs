//! The server side of a filesystem read: what the daemon sends.
//!
//! [`response`] is the whole of it. The daemon streams the file and
//! is done; it opens no channels of its own for a read.

pub mod response;
