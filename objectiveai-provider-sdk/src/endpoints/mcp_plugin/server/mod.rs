//! The server side of an MCP plugin: what a provider sends.
//!
//! [`response`] is what comes back on channel `0` once the plugin
//! runs. [`channel_response`] answers the channel a caller opens to
//! call it.
//!
//! No `channel_request` yet. A provider will need one to ask a caller
//! for a
//! [`Client`](crate::shared::container::request::ImageType::Client)
//! image, exactly as a laboratory creation does; it is not written.

pub mod channel_response;
pub mod response;
