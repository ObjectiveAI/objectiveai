//! Asking for one mounted FILE the provider is missing.
//!
//! The request's `file_mounts` names files by identity —
//! `f1:<size>:<base64url sha256 of the bytes>`, the size riding the
//! identity so a provider can judge the weight before fetching
//! anything — and the content itself lives with the client. A
//! provider missing one opens a channel with a [`Request`] naming
//! the identity, and the client answers with the bytes themselves:
//! [`fetch_file::Frame`](crate::endpoints::agentic_loop::run::client::channel_response::fetch_file::Frame)s
//! of at most
//! [`CHUNK_SIZE`](crate::CHUNK_SIZE)
//! each, every frame appending to the one file, then the finish.
//!
//! # By identity, not by path
//!
//! The request carries no path. A path is where the CALLER wants the
//! content mounted and may name different content tomorrow; the
//! identity IS the content. The client indexes its store however it
//! likes — what it must answer to is the identity.
//!
//! # Absence is the empty finish
//!
//! A client without the identity sends no frames and finishes the
//! channel — this protocol's deliberate could-not-serve. There is no
//! error vocabulary on this exchange: nothing an error could say
//! would change what the provider does next, which is not run the
//! agent.
//!
//! # This endpoint's own, not [`shared`](crate::shared)
//!
//! Nothing but an agentic loop fetches mounts, so this lives where
//! it is used — a shape moves to `shared` when a second endpoint
//! needs it, not before.

mod request;

pub use request::*;
