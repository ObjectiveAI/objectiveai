//! Container creation request data.
//!
//! [`Frame`] is what a caller hands a provider to get a container.
//! [`Image`] says where the image comes from and [`Mount`] is how a
//! caller asks for a directory inside one.

mod frame;
mod image;
mod mount;

pub use frame::*;
pub use image::*;
pub use mount::*;
