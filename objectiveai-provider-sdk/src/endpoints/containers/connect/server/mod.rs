//! The server side of a connection: what a provider sends.
//!
//! [`channel_response`] answers the channels the connector opens into
//! the container. [`response`] is what comes back on channel `0`.
//!
//! There is no `channel_request`. A provider asks a connector for
//! nothing — the image was somebody else's problem, and so is deciding
//! who may attach.

pub mod channel_response;
pub mod response;
