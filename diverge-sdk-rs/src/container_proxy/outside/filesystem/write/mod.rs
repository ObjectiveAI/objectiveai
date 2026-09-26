//! Writing one file into the container.
//!
//! Split by who SENDS: the destination and the content go in
//! [`client`], whether it landed comes back in [`server`].
//!
//! The server names the file. The proxy opens a channel asking for
//! the content; the server answers it with the bytes, in pieces, and
//! finishes; the proxy puts the file in place and answers
//! [`Written`](server::response::Frame::Written) on channel `0`, or
//! an error, and finishes.
//!
//! # Why a write is two exchanges
//!
//! Because only a responder can end a channel.
//! [`ChannelResponseFinish`](crate::wire::frame::client::ClientFrame::ChannelResponseFinish)
//! is sent by whoever is ANSWERING, so a side that is asking has no
//! way to say "that was the last one". A server streaming content
//! into its own scope would have to invent an end marker in the
//! payload, and its request would become stateful.
//!
//! Inverting the second exchange fixes that with machinery that
//! already exists — the shape
//! [`write_path`](crate::shared::containers::write_path) and
//! [`write_bytes`](crate::shared::containers::write_bytes) give the
//! provider protocol. Here the second channel needs no id of its own:
//! it is opened on the write's scope, and the scope says which write.
//!
//! # What lands
//!
//! The file, created or replaced, as the bytes arrived; parents are
//! not created — the request names a file, not a tree, and a missing
//! parent is an error. The proxy writes the content beside its
//! destination and moves it over in one rename when the content
//! finishes, so the destination is either what it was or the whole
//! new file, never the half between. A content channel that ends in
//! an error is a write the proxy abandons, and it discards what it
//! wrote.

pub mod client;
pub mod server;
