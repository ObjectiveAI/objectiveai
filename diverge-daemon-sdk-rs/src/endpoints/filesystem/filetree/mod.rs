//! Watching a directory of the daemon's host.
//!
//! A client names a directory by its absolute host path; the daemon
//! sends the tree under it, whole, and then one frame per change, as
//! it happens, for as long as the client keeps the scope. Split by who
//! SENDS, as everywhere else: the ask and the cancel are in
//! [`client`], the frames in [`server`].
//!
//! # Live, as a container's tree is
//!
//! The frames are a [`filetree`](diverge_provider_sdk::shared::filetree)
//! stream over the directory named: a
//! [`Snapshot`](diverge_provider_sdk::shared::filetree::response::Frame::Snapshot)
//! first, then every change, a fresh snapshot whenever the daemon has
//! lost track, every path relative to the directory. What a
//! container's filetree channel says of its frames holds here, the
//! directory standing where the container's root stands. A directory
//! the daemon cannot watch is in the tree with `changes` false, and no
//! change under it is sent until the next snapshot.
//!
//! The scope stays open until the client
//! [cancels](client::channel_request::Frame::Cancel), closes the scope,
//! or the watch dies, which is the daemon's error, last.

pub mod client;
pub mod server;
