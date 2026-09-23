//! The server side of a logs read: what the daemon sends.
//!
//! [`response`] is the whole of it. The daemon answers with what
//! matches and is done, or stays open and keeps answering; it opens
//! no channels of its own, so there is no `request` here.

pub mod response;
