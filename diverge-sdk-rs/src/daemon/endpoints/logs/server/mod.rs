//! The server side of a logs read: what the daemon sends.
//!
//! [`response`] is the whole of it. The daemon answers with what
//! matches, and watching goes on answering; it opens no channels of
//! its own, so there is no `request` here.

pub mod response;
