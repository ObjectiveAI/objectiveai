//! What the server sends back on a mount, one module per kind of ask
//! the proxy opens.
//!
//! Each is the caller's answer, relayed: what the file or directory
//! holds, what an entry is, whether a change took — an alias of the
//! shape [`shared::containers::fuse`](crate::shared::containers::fuse)
//! defines.

pub mod list;
pub mod mkdir;
pub mod read;
pub mod remove;
pub mod rename;
pub mod stat;
pub mod write;
