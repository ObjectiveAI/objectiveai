//! The container's filesystem, as the server sees it: the tree
//! watched, a subtree served ask by ask, and the FUSE mounts — files
//! and directories made on the server's request, their contents the
//! caller's.

pub mod mount;
pub mod served;
pub mod tree;
