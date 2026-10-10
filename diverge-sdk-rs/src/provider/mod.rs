//! The provider protocol: what a caller asks a provider for, and what
//! comes back.
//!
//! This module is the **normative artifact** of the provider
//! specification. The types defined here are not a description of the
//! protocol written alongside an implementation — they are the
//! protocol. Prose documents the requirements a provider must satisfy;
//! it does not define the messages. Where the two disagree, this
//! crate is correct and the prose is a bug. These definitions are the
//! specification's machine-readable half: nothing describing the
//! protocol is hand-authored alongside them, so nothing can drift
//! from what the protocol actually is.
//!
//! [`endpoints`] is the request vocabulary — the eighteen scopes a
//! caller opens, each with what it sends and what comes back — over
//! the [`wire`](crate::wire) and the [`shared`](crate::shared)
//! shapes. [`client`] is what a caller supplies while a container
//! runs: the answerers a provider asks for the things that live with
//! the caller. The provider's half — the dispatch in front of every
//! endpoint's handler, and the traits a provider implements — is the
//! `diverge-provider` crate's `protocol`, built on this crate's
//! frames and its [`wire::server`](crate::wire::server).

pub mod client;
pub mod endpoints;
