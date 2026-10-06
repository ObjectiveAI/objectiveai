//! The server side of a delete: what the daemon sends.
//!
//! [`response`] is the whole of it. The daemon answers and is done; it
//! opens no channels of its own for this, so there is no `request`
//! here.

pub mod response;
