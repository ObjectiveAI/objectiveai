//! Watching one of the directories a provider offers.
//!
//! A caller names a directory from [`list`](super::list) and gets its
//! tree, then keeps getting the changes to it for as long as the scope
//! stays open. Split by who SENDS: the name goes in [`client`], the
//! stream comes back in [`server`].
//!
//! The stream itself is [`filetree`](crate::shared::filetree)'s, not a second
//! description of the same thing.

pub mod client;
pub mod server;
