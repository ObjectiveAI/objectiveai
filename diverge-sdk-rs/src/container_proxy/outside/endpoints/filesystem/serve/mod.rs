//! Serving a subtree of the container live: the FUSE asks, answered
//! from the container's own filesystem.
//!
//! Split by who SENDS: the subtree, the asks, the stop and the tree
//! go in [`client`]; whether it is served, and every answer, come
//! back in [`server`].
//!
//! The server names a directory of the container, as components from
//! the root. The proxy answers [`Serving`](server::response::Frame::Serving)
//! on channel `0`, or an error, and from then on answers every
//! channel the server opens on the scope: the nine asks of
//! [`fuse`](crate::shared::containers::fuse) — a stat, a piece read, a
//! piece written, a truncate, a setattr, a listing, a removal, a
//! rename, a mkdir — each from the subtree, with no mount id, since
//! the scope is the subtree; and the tree of the subtree as a
//! [`filetree`](crate::shared::filetree) stream. The server's stop ends
//! the scope.
//!
//! # The far end of a container serve
//!
//! This is the last hop of a
//! [`containers::serve`](crate::provider::endpoints::containers::serve) on
//! the provider wire: the provider opens one of these on the
//! container's proxy, carrying the caller's path, and relays every
//! channel the caller opens here as it came and every answer back as
//! it came. The channel request is the volume serve's very frame, so
//! the relay forwards bytes.
//!
//! # The mount scope, the other way round
//!
//! A [`fuse::mount`](super::super::fuse::mount) is the proxy asking: the
//! kernel wants a file, and the proxy opens a channel to the server
//! for it. This is the server asking: it wants a file of the
//! container, and it opens a channel to the proxy for it. The asks are
//! the same frames in both directions.
//!
//! # What is answered
//!
//! Every path in an ask is relative to the subtree, `/`-separated and
//! empty for the subtree's root. Every mutation lands in the
//! container, where the program beside the proxy sees it: there is
//! no layer and no read-only answer. Under `/proc`, `/sys` and `/dev`
//! nothing is answered — a stat or a list there is `Missing`, a
//! mutation an error — and a subtree that lies under one of them is
//! refused at the opening, as is a path that is not a directory of
//! the container.

pub mod client;
pub mod server;
