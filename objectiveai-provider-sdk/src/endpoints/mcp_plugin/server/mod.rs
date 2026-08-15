//! The server side of an MCP plugin: what a provider sends.
//!
//! [`response`] is what comes back on channel `0` once the plugin
//! runs.
//!
//! No `channel_request` and no `channel_response` yet. A provider will
//! need the first to ask a caller for a
//! [`Client`](crate::shared::container::request::ImageType::Client)
//! image, exactly as a laboratory creation does, and the second to
//! answer whatever channels a caller opens to call the plugin. Neither
//! is written.

pub mod response;
