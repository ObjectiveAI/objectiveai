//! The server side of a filesystem filetree: what the daemon sends.
//!
//! [`response`] is the whole of it. The daemon sends the snapshot and
//! then every change; it opens no channels of its own.

pub mod response;
