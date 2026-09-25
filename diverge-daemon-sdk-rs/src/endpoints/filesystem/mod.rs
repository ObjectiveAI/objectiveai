//! The daemon's host filesystem: read a file, write one, watch a tree.
//!
//! A client names a path on the daemon's own host — absolute, as the
//! host writes one, the same way a FUSE mount names its
//! [`daemon_path`](crate::endpoints::agents::create::client::request::FuseMount::daemon_path)
//! — and the daemon serves it: [`read`] streams a file out, [`write`](mod@write)
//! streams one in and puts it in place whole, and [`filetree`] sends
//! the tree under a directory and then every change to it, live,
//! until the client cancels. Nothing here names an agent or a
//! provider: this is the host the daemon runs on, and the daemon is
//! the only party that touches it, as it is for a FUSE mount it
//! serves into a container.
//!
//! The vocabulary is the provider protocol's: a piece of a file, a
//! file that landed, and a filetree frame mean here what they mean on
//! a container's channels and on a volume, and the shared types are
//! used as they are.

pub mod filetree;
pub mod read;
pub mod write;
