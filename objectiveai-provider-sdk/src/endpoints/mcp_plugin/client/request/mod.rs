//! MCP plugin request data.
//!
//! [`Frame`] is what a caller hands a provider to get a plugin
//! running. [`Identity`] says on whose behalf, and [`ImageType`] says
//! who produces the image.

mod frame;
mod identity;
mod image_type;

pub use frame::*;
pub use identity::*;
pub use image_type::*;
