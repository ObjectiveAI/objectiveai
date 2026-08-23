//! The server side of a laboratory run: what a provider sends.
//!
//! [`channel_request`] is what it opens channels to ask the caller for
//! while pulling the image. [`channel_response`] answers the channels
//! the caller opens into the container. [`response`] is what comes
//! back on channel `0` once it runs.

//!
//! # And, behind the `server` feature, [`handle`]
//!
//! It deploys the container, registers it under the id it mints, and
//! then serves the caller, the container and every connector's
//! authorization at once, for as long as the laboratory runs.

pub mod channel_request;
pub mod channel_response;
pub mod response;

#[cfg(feature = "server")]
pub mod handle;
