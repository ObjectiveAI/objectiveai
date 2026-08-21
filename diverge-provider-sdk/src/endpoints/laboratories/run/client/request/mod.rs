//! Laboratory run request data.
//!
//! [`Frame`] is what a caller hands a provider to get a container.
//! The image is [`Image`](crate::shared::container::request::Image)
//! and a volume is a
//! [`Mount`](crate::shared::container::request::Mount); both live in
//! [`shared`](crate::shared) because they mean the same thing for
//! every kind of container.

mod frame;

pub use frame::*;
