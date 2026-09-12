//! The `/filesystem/tree` path: the container's filesystem, streamed to the
//! server.
//!
//! The one stream path, and the one where the server is the reader
//! and the container the source. The proxy watches the container's
//! filesystem from its root and sends one [`Frame`](response::Frame)
//! per event: first a snapshot of the whole tree, then one delta per
//! change, for as long as the connection lives. The server sends
//! exactly one message, first — the [`request::Request`], naming
//! what the tree leaves out — and nothing after it; a further binary
//! message from it is a peer speaking something else, at which the
//! proxy closes.
//!
//! ```text
//! server → container:  [request JSON]                             once
//! container → server:  [0][postcard-encoded filetree frame…]   per event
//!                   or [1][message…]                            last, then the close
//! ```
//!
//! One kind byte, then the frame: a [`shared::filetree`](crate::shared::filetree)
//! response frame encoded as that module encodes it — postcard,
//! high-volume, relayed to nobody in that form — or, once, why there
//! will be no more of them. The byte per frame is the price of a
//! stream that can say why it ended, and it is small next to the
//! paths a frame carries.
//!
//! # What is an error, and what is not
//!
//! An [`Error`](response::Frame::Error) is the watch failing to
//! exist: the watcher could not be made, the root could not be
//! watched at all, or the walk died. It is the last message, and the
//! close follows. Everything short of that is not an error and is
//! reported in the tree itself: a corner that could not be watched
//! carries `changes` false, a corner that could not be read is absent,
//! and lost events bring a fresh snapshot — see below.
//!
//! # Every connection is a fresh subscription
//!
//! A connection opens with a snapshot, so there is nothing to resume:
//! the server connects, says what to leave out, and reads. The path
//! accepts as many connections as the server opens, each a watch of
//! its own starting whole; one at a time is the expected use, and
//! more is merely allowed. A connection the proxy could not begin —
//! the watch would not arm, the root would not be watched — answers
//! one [`Error`](response::Frame::Error) and closes, and the server
//! starts over when it likes. A connection closed before the request,
//! or whose request will not decode, is closed with nothing before
//! it: the server sent nothing this is, and there is no watch to say
//! anything about.
//!
//! # What the tree leaves out
//!
//! `/proc`, `/sys` and `/dev` — the pseudo-filesystems — and the
//! MOUNTS the server placed, every one, which it names in the
//! [`request::Request`] that opens the watch. An ignored path does
//! not exist as far as the stream is concerned: absent from the
//! snapshot, never watched, an event under it dropped.
//!
//! Different from a corner the proxy could not WATCH — a subtree the
//! watch limit ran out on, or that refused registration. That stays
//! in the tree, walked as everything else is, with its directory's
//! `changes` false: what is there is what the walk found, and
//! nothing under it is reported until the next snapshot.
//!
//! # A snapshot may come again
//!
//! A watch that lost events — its queue overflowed under a burst —
//! no longer knows what changed, and deltas it cannot know would be
//! lies. The proxy re-walks and sends a fresh
//! [`Snapshot`](crate::shared::filetree::response::Frame::Snapshot)
//! on the same connection, which replaces the tree whole; the fold
//! is built for it.
//!
//! # What the frames promise
//!
//! What [`shared::filetree::response`](crate::shared::filetree::response)
//! states: a snapshot first, deltas carrying whole nodes and never
//! patches, no move — a rename is a removal and an insertion — paths
//! as components relative to the root, and a reader that folds them
//! with [`Root::update`](crate::shared::filetree::response::Root::update)
//! and tolerates a frame it cannot apply.

pub mod request;
pub mod response;

#[cfg(feature = "server")]
pub mod execute;
