//! The server side of a query: what the daemon sends.
//!
//! [`response`] is the whole of it. The daemon answers with the items
//! and is done; it opens no channels of its own, so there is no
//! `request` here.

pub mod response;
