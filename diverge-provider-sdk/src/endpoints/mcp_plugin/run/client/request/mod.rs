//! MCP plugin request data.
//!
//! [`Frame`] is what a caller hands a provider to get a plugin
//! running, and [`Identity`] says on whose behalf. The image is
//! [`Image`](crate::shared::container::request::Image), shared with
//! every other kind of container.

mod frame;
mod identity;

pub use frame::*;
pub use identity::*;
