//! The channels the proxy opens on the server during a mount: the
//! mount's nine asks. See [`Frame`].
//!
//! What each carries is defined here, beside the frame, because it
//! rides no other wire with the mount left off: the provider's
//! channel toward the caller carries the same asks with the mount's
//! id in front, and here the scope is the mount — as it is the
//! volume on a `volumes::serve` scope, which carries these very
//! frames. [`Path`] is the whole ask where nothing but the path is
//! carried; [`Read`], [`Write`] and [`Rename`] prefix the path
//! because something follows it; [`Truncate`] and [`Setattr`] put
//! their fixed bytes before it.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod frame;
mod path;
mod prefixed;
mod read;
mod rename;
mod setattr;
mod truncate;
mod write;

pub use frame::*;
pub use path::*;
pub use read::*;
pub use rename::*;
pub use setattr::*;
pub use truncate::*;
pub use write::*;
