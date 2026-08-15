//! Container creation request data.
//!
//! [`Frame`] is what a caller hands a provider to get a container, and
//! [`Mount`] is how a caller asks for a volume inside one. The image
//! is [`Image`](crate::shared::container::request::Image), which lives
//! in [`shared`](crate::shared) because it means the same thing for
//! every kind of container.

mod frame;
mod mount;

pub use frame::*;
pub use mount::*;
