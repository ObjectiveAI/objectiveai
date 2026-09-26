//! The Diverge SDK: one crate for everything that crosses a Diverge
//! wire.
//!
//! | module | what it is |
//! |--------|------------|
//! | [`wire`] | one WebSocket, nine-byte frames, scopes and channels, the encode/decode contract, and the frame-level cores of both halves — what every protocol here is spoken over |
//! | [`shared`] | the shapes more than one endpoint is made of: the one error, what a container asks and is answered, filetrees, MCP |
//! | [`provider`] | the provider protocol: its sixteen endpoints, the caller half and the provider half — the normative artifact of the specification |
//! | [`daemon`] | the daemon protocol: its endpoints, over the same wire |
//! | [`container_proxy`] | the proxy beside every container's program, from both sides: [`outside`](container_proxy::outside), the WebSocket a provider opens into it; [`inside`](container_proxy::inside), the loopback the program dials it on |
//!
//! # Nothing is optional
//!
//! There are no features. Every half of every protocol compiles for
//! every dependent — a caller carries the provider half it never
//! runs, a program inside a container carries the wire it never
//! speaks — so that nothing a reader names is behind a flag and no
//! two builds of this crate disagree about what exists. What that
//! costs is compile time; what it buys is one crate that is the same
//! crate everywhere.
//!
//! # More than one wire format
//!
//! There is no single serialization for these protocols, and there is
//! no flag anywhere that says which one applies. Each payload states
//! its own, by implementing [`Encode`](wire::encode::Encode) and
//! [`Decode`](wire::decode::Decode) — so the format is a property of
//! the type, settled where the type is defined, and no caller chooses
//! it.
//!
//! It differs by channel because the obligations differ. What crosses
//! an MCP channel is JSON, because that channel relays JSON-RPC and
//! has to hand it on byte-identical. Nothing else carries that
//! obligation, and paying JSON's cost for a filetree event would be
//! paying it for nothing.
//!
//! # What a schema cannot say
//!
//! Every type here describes one message. A stream is a *sequence* of
//! messages, and its ordering and cardinality rules are not
//! expressible in JSON Schema — a schema constrains a document, not a
//! trajectory. Rules of that kind belong in the prose specification,
//! and are enforced by construction in the halves here so an author
//! cannot violate a requirement that only a document states.

pub mod container_proxy;
pub mod daemon;
pub mod provider;
pub mod shared;
pub mod wire;

/// The most bytes one frame of content carries, anywhere in this
/// crate — a container file read out or written in, a transfer's
/// chunk, a blob's piece. The SENDER's rule alone: content
/// larger than this leaves as adjacent frames, and every receiver is
/// chunk-naive — same content, next frame, append — and never
/// measures. One number, at the root, so no two paths can disagree.
pub const CHUNK_SIZE: usize = 2 * 1024 * 1024;
