//! The server side of a filesystem listing: what a provider sends.
//!
//! [`response`] is the whole of it. A provider answers and is done; it
//! opens no channels of its own for a question this small.

pub mod response;
