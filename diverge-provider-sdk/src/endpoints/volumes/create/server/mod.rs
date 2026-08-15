//! The server side of a volume creation: what a provider sends.
//!
//! [`response`] is the whole of it. A provider makes the volume,
//! answers, and is done; it opens no channels of its own, because
//! there is nothing it needs from the caller that the request did not
//! already carry.

pub mod response;
