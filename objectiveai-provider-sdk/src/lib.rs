//! Wire types for the ObjectiveAI provider API.
//!
//! This crate is the **normative artifact** of the provider
//! specification. The types defined here are not a description of the
//! protocol written alongside an implementation — they are the
//! protocol. Prose documents the requirements a provider must satisfy;
//! it does not define the messages. Where the two disagree, this crate
//! is correct and the prose is a bug.
//!
//! The JSON Schema corpus is generated from these types, and the
//! per-language SDKs are generated from that. Nothing downstream is
//! hand-authored, so nothing downstream can drift.
//!
//! # Scope
//!
//! Types only — no transport, no client, no server. A provider
//! implementation binds these types to a transport; the frameworks
//! that make that easy live in separate crates. Keeping this crate
//! free of runtime concerns is what lets it be depended on by both
//! sides of the protocol, and by tools that only ever inspect it.
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
//! Empty by design. The types land as the provider API is defined.
