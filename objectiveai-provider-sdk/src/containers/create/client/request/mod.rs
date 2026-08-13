//! Container creation request data.
//!
//! [`Frame`] is what a caller hands a provider to get a container, and
//! [`Mount`] is how it asks for a directory inside one.

mod frame;
mod mount;

pub use frame::*;
pub use mount::*;
