//! Writing one file into a volume.
//!
//! A caller names a volume and a destination in it; a provider asks
//! for the content on a channel of its own, puts the file in place
//! whole, and says so. Split by who SENDS, as everywhere else: the
//! destination and the content are in [`client`], the answer and the
//! ask for the content in [`server`].
//!
//! # The volume at rest
//!
//! A write takes the volume to itself, as an [`edit`](super::edit)
//! does: nothing is written while any container has the volume, and
//! no run takes it while the write runs. The file lands or it does
//! not, and no container sees the half between.
//!
//! # Why a write is two exchanges
//!
//! Because only a responder can end a channel.
//! [`ChannelResponseFinish`](crate::frame::client::ClientFrame::ChannelResponseFinish)
//! is sent by whoever is ANSWERING, so a side that is asking has no
//! way to say "that was the last one". A client streaming content
//! into its own scope would have to invent an end marker in the
//! payload, and its request would become stateful.
//!
//! Inverting the second exchange fixes that with machinery that
//! already exists — the shape a container's
//! [`write_path`](crate::shared::containers::write_path) and
//! [`write_bytes`](crate::shared::containers::write_bytes) have. Here
//! the second channel needs no id of its own: it is opened on the
//! write's scope, and the scope says which write.
//!
//! # What lands
//!
//! The file, created or replaced, as the bytes arrived, with every
//! missing parent directory made: a caller seeds a fresh volume
//! without a container to make its directories first. The
//! destination is either what it was or the whole new file, never
//! the half between. A content channel that ends in an error is a
//! write the provider abandons, and the destination is as it was.

pub mod client;
pub mod server;
