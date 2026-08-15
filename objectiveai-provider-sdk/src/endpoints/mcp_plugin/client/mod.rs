//! The client side of an MCP plugin: what a client sends.
//!
//! [`request`] opens the scope. [`channel_request`] calls the plugin
//! once it runs.
//!
//! No `channel_response` yet. A caller serving its own image will need
//! one, exactly as a laboratory creation's does; it is not written.

pub mod channel_request;
pub mod request;
