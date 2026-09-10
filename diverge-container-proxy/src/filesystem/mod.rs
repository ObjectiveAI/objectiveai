//! The `/filesystem/*` paths, served: the tree watched, one file
//! read, one file written — and the FUSE mounts, files and
//! directories, made at the proxy's start, their contents the
//! caller's.

pub mod read;
pub mod tree;
pub mod write;

mod mount;

pub use mount::*;
