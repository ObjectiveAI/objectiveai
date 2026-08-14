//! The channels a connector opens on a provider.
//!
//! One, and it is the same one a creation opens: the container runs on
//! the provider, so its MCP server is the thing a connector cannot
//! dial.

mod frame;

pub use frame::*;
