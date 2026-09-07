//! Wire types for the Diverge provider API.
//!
//! This crate is the **normative artifact** of the provider
//! specification. The types defined here are not a description of the
//! protocol written alongside an implementation — they are the
//! protocol. Prose documents the requirements a provider must satisfy;
//! it does not define the messages. Where the two disagree, this crate
//! is correct and the prose is a bug.
//!
//! These definitions are the specification's machine-readable half.
//! Nothing describing the protocol is hand-authored alongside them, so
//! nothing can drift from what the protocol actually is.
//!
//! # Scope
//!
//! Types only, by default — no transport, no client, no server. A
//! provider implementation binds these types to a transport. Keeping
//! the default build free of runtime concerns is what lets it be
//! depended on by both sides of the protocol, and by tools that only
//! ever inspect it.
//!
//! Two features add halves that do: `server` adds [`server`] and
//! `client` adds [`client`]. Both are off unless asked for, so nothing
//! above changes for anyone who does not ask.
//!
//! [`connection`] appears with either, and carries both kinds of socket
//! under either. Which end dialled is a fact about TCP, not about the
//! protocol: a provider usually waits to be dialled and sometimes
//! dials a caller it cannot otherwise reach, and a caller can
//! perfectly well be dialled into. The frames are the same frames
//! whichever way round it went.
//!
//! # More than one wire format
//!
//! There is no single serialization for this protocol, and there is no
//! flag anywhere that says which one applies. Each payload states its
//! own, by implementing [`Encode`](encode::Encode) and
//! [`Decode`](decode::Decode) — so the format is a property of the
//! type, settled where the type is defined, and no caller chooses it.
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
//! trajectory. Rules of that kind belong in the prose specification
//! with RFC 2119 keywords, and are enforced by construction in the
//! provider frameworks so an author cannot violate a requirement that
//! only a document states.
//!
//! # Status
//!
//! Under construction. The types land as the provider API is defined.

pub mod agentic_loop_container;
#[cfg(feature = "client")]
pub mod client;
#[cfg(any(feature = "client", feature = "server"))]
pub mod connection;
pub mod container_proxy;
pub mod decode;
pub mod encode;
pub mod endpoints;
pub mod frame;
pub mod mcp_proxy;
pub mod postgres_proxy;
#[cfg(feature = "server")]
pub mod server;
pub mod shared;

/// The most bytes one frame of content carries, anywhere in this
/// crate — a fetched file's piece, a container file read out or
/// written in, a transfer's chunk. The SENDER's rule alone: content
/// larger than this leaves as adjacent frames, and every receiver is
/// chunk-naive — same content, next frame, append — and never
/// measures. One number, at the root, so no two paths can disagree.
pub const CHUNK_SIZE: usize = 2 * 1024 * 1024;
