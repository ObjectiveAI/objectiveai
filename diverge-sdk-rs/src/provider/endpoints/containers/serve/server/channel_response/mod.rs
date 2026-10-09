//! What the provider sends back on a container serve, one module per
//! kind of channel the caller opens.
//!
//! Each ask's is the shared vocabulary's own answer — what the file
//! or the directory holds, what an entry is, whether a change took —
//! an alias of the shape
//! [`shared::containers::fuse`](crate::shared::containers::fuse)
//! defines, answered from the container by its proxy and relayed as
//! it came. [`filetree`] is the one stream: the subtree as the
//! container holds it, then every change in it.

pub mod filetree;
pub mod list;
pub mod mkdir;
pub mod read;
pub mod remove;
pub mod rename;
pub mod setattr;
pub mod stat;
pub mod truncate;
pub mod write;
