//! The server side of an announce: what a provider sends.
//!
//! [`response`] is the whole of it: the one refusal. A provider sends
//! nothing while an announce stands, and opens no channel of its own
//! on one; the one channel on it is the caller's cancel.

pub mod response;
