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
//! Types only — no transport, no client, no server. A provider
//! implementation binds these types to a transport; the frameworks
//! that make that easy live in separate crates. Keeping this crate
//! free of runtime concerns is what lets it be depended on by both
//! sides of the protocol, and by tools that only ever inspect it.
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

pub mod auth;
pub mod decode;
pub mod encode;
pub mod endpoints;
pub mod frame;
pub mod shared;
