//! Watching one of the directories a provider offers.
//!
//! A caller names a directory from [`list`](super::list) and gets its
//! tree, then keeps getting the changes to it until it says stop. Split
//! by who SENDS: the name and the stop go in [`client`], the stream
//! comes back in [`server`].
//!
//! The stop is what makes this the only volume scope with two things
//! to send. Every other one ends by answering; a watch has to be ended.
//!
//! The stream itself is [`filetree`](crate::shared::filetree)'s, not a second
//! description of the same thing.

pub mod client;
pub mod server;
