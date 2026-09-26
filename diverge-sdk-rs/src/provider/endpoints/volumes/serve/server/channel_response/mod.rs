//! What the provider sends back on a serve, one module per kind of
//! ask the caller opens.
//!
//! Each is the shared vocabulary's own answer — what the file or the
//! directory holds, what an entry is, whether a change took — an
//! alias of the shape [`shared::containers::fuse`](crate::shared::containers::fuse)
//! defines, answered from the volume.

pub mod list;
pub mod mkdir;
pub mod read;
pub mod remove;
pub mod rename;
pub mod setattr;
pub mod stat;
pub mod truncate;
pub mod write;
