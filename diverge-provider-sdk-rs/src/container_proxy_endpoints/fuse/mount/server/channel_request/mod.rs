//! The channels the proxy opens on the server during a mount: the
//! mount's seven asks. See [`Frame`].
//!
//! What each carries is defined here, beside the frame, because it
//! rides no other wire: the provider's channel toward the caller
//! carries the same asks with the mount's id in front, and here the
//! scope is the mount. [`Path`] is the whole ask where nothing follows
//! the path; [`Write`] and [`Rename`] are the two where something
//! does.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod frame;
mod path;
mod prefixed;
mod rename;
mod write;

pub use frame::*;
pub use path::*;
pub use rename::*;
pub use write::*;
