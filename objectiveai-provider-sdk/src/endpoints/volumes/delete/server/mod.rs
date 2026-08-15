//! The server side of a volume deletion: what a provider sends.
//!
//! [`response`] is the whole of it. A provider destroys the volume,
//! answers, and is done.

pub mod response;
