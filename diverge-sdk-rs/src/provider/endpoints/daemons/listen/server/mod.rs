//! The server side of a listen: what a provider sends.
//!
//! [`response`] is the whole of it: a message, or a refusal. A
//! provider opens no channel of its own on a listen; the one channel on
//! it is the caller's cancel.

pub mod response;
