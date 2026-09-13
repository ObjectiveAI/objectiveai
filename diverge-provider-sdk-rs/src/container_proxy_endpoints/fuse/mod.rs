//! FUSE mounts, from the proxy's side: one scope per mount, and the
//! mount's asks on it.
//!
//! [`mount`] is the scope. What a mount IS — a file or a tree the
//! caller serves live, what each ask means, what the program sees —
//! is [`shared::containers::fuse`](crate::shared::containers::fuse),
//! stated once for both wires; this is only how the last hop of it is
//! carried.

pub mod mount;
