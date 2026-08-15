//! Container creation request data.
//!
//! [`Frame`] is what a caller hands a provider to get a container, and
//! [`Mount`] is how a caller asks for a directory inside one. Who
//! produces the image is
//! [`ImageType`](crate::shared::container::request::ImageType), which
//! lives in [`shared`](crate::shared) because it means the same thing
//! for every kind of container.

mod frame;
mod mount;

pub use frame::*;
pub use mount::*;
