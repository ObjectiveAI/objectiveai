//! The server side of a message: what a provider sends.
//!
//! [`response`] is the whole of it: the one answer. A provider opens
//! no channel of its own on a message; the one channel on it is the
//! caller's cancel.

pub mod response;
