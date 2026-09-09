//! The `/filesystem/*` paths, served: the tree watched, one file
//! read, one file written — and the FUSE mounts, one file each, made
//! at the proxy's start, their bytes the caller's.

pub mod read;
pub mod tree;
pub mod write;

mod mount;

pub use mount::*;
