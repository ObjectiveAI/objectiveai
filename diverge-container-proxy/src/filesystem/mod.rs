//! The `/filesystem/*` paths, served: the tree watched, one file
//! read, one file written — and the file mounts, one FUSE file each,
//! made at the proxy's start.

pub mod read;
pub mod tree;
pub mod write;

mod mount;

pub use mount::*;
