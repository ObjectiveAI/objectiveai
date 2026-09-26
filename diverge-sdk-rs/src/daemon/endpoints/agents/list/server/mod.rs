//! The server side of a list: what the daemon sends.
//!
//! [`response`] is the whole of it. The daemon answers with the
//! agents, one each, and is done; it opens no channels of its own,
//! so there is no `request` here.

pub mod response;
