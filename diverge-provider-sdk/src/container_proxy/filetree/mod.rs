//! The `/filetree` path: the container's filesystem, streamed to the
//! server.
//!
//! The one path where the server is the reader and the container the
//! source. The proxy watches the container's filesystem from its
//! root and sends one [`Frame`](container::Frame) per event: first a
//! snapshot of the whole tree, then one delta per change, for as
//! long as the connection lives. The server sends NOTHING on this
//! path; a message from it is a peer speaking something else, and
//! the proxy closes.
//!
//! ```text
//! container → server:  [postcard-encoded filetree frame…]
//! ```
//!
//! No header at all: the container sends exactly one kind of frame,
//! so there is no type byte to spend, and the frame is the
//! [`shared::filetree`](crate::shared::filetree) response frame
//! encoded as that module encodes it — postcard, high-volume,
//! relayed to nobody in that form.
//!
//! # Every connection is a fresh subscription
//!
//! A connection opens with a snapshot, so there is nothing to resume
//! and nothing to ask: the server connects and reads. The proxy
//! multiplexes one watch into as many subscriptions as connections
//! it has served, one at a time, each starting whole.
//!
//! # What the frames promise
//!
//! What [`shared::filetree::response`](crate::shared::filetree::response)
//! states: one `Snapshot`, then deltas carrying whole nodes and never
//! patches, paths as components relative to the root, and a reader
//! that folds them with [`Root::update`](crate::shared::filetree::response::Root::update)
//! and tolerates a frame it cannot apply.

pub mod container;

mod error;

pub use error::FrameError;
