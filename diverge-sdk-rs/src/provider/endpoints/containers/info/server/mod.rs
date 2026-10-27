//! The server side of an image's info: what a provider sends.
//!
//! [`response`] is the whole of it. A provider answers and is done; it
//! opens no channels for a question this small.

pub mod response;
