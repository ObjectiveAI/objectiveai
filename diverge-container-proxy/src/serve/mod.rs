//! The scopes the server opens past the begin, each served to its
//! end: one FUSE mount, one tree watched, one file read, one file
//! written.

mod mount;
mod read;
mod tree;
mod write;

pub use mount::*;
pub use read::*;
pub use tree::*;
pub use write::*;
