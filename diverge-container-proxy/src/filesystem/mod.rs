//! The container's filesystem, as the server sees it: the tree
//! watched, and the FUSE mounts — files and directories made on the
//! server's request, their contents the caller's.

pub mod mount;
pub mod tree;
