//! The scopes the server opens past the begin, each served to its
//! end: one FUSE mount, one tree watched, one file read, one file
//! written, one subtree served.

mod mount;
mod read;
mod served;
mod tree;
mod write;

pub use mount::*;
pub use read::*;
pub use served::*;
pub use tree::*;
pub use write::*;
