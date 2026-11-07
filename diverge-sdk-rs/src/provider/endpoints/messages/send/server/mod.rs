//! The server side of a send: what a provider sends.
//!
//! [`response`] is the whole of it: the one answer. A provider opens
//! no channel of its own on a send; the one channel on it is the
//! caller's cancel.

pub mod response;
