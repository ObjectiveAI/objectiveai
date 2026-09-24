//! Seeing what a volume holds: one snapshot of its tree.
//!
//! A caller names a volume and a subtree of it, empty for the whole;
//! a provider answers with the tree as it is, once, and finishes.
//! Split by who SENDS, as everywhere else: the ask is in [`client`],
//! the answer in [`server`].
//!
//! # A snapshot, not a watch
//!
//! The tree a container's
//! [`filetree`](crate::shared::containers::filetree) channel carries
//! is live: a snapshot, then every change, for as long as the channel
//! lives. This is the snapshot alone. Nothing follows it, no change is
//! ever reported, and every directory in it says so — its `changes`
//! is `false`. A caller that wants to know what changed asks again,
//! and compares.
//!
//! # The volume at rest
//!
//! A filetree takes the volume to itself, as a [`stat`](super::stat)
//! does: nothing is walked while any container has the volume, and no
//! run takes it while the walk runs. So the snapshot is the volume as
//! it is, whole, with nothing moving underneath the walk — which is
//! what makes it a snapshot rather than a walk that lies about its
//! moment.

pub mod client;
pub mod server;
