//! Container creation request data.
//!
//! [`Frame`] is what a caller hands a provider to get a container.
//! [`ImageType`] says who produces the image and [`Mount`] is how a
//! caller asks for a directory inside one.

mod frame;
mod image_type;
mod mount;

pub use frame::*;
pub use image_type::*;
pub use mount::*;
