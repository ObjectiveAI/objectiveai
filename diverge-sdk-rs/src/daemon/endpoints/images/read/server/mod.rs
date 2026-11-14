//! The server side of the daemon's read scope: what the daemon sends.
//!
//! [`response`] is the one frame it sends on the main stream, first:
//! the answer to whether it can supply the image. After a held, nothing
//! more comes on channel `0` until the finish; after anything else, the
//! finish follows at once. [`channel_response`] is the three answers,
//! one module per kind of channel the caller opens, each the provider
//! scope's own.

pub mod channel_response;
pub mod response;
