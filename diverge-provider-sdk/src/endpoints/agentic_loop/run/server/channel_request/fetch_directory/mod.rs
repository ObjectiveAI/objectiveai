//! Asking for one mounted DIRECTORY the provider is missing.
//!
//! The request's `directory_mounts` names directories by identity —
//! `d1:<total size>:<base64url sha256 of the manifest>`, the total
//! riding the identity so a provider can judge the weight before
//! fetching anything — and the content itself lives with the client.
//! A provider missing one opens a channel with a [`Request`] naming
//! the identity, and the client answers with the directory's files:
//! one
//! [`fetch_directory::Frame`](crate::endpoints::agentic_loop::run::client::channel_response::fetch_directory::Frame)
//! per file — or several per file, adjacently, when a file exceeds
//! [`CHUNK_SIZE`](crate::CHUNK_SIZE)
//! — then the finish.
//!
//! # By identity, not by path
//!
//! The request carries no mount path and no name. A path is the
//! caller's placement, a name the caller's label; the identity IS
//! the content, and the client indexes its store however it likes —
//! what it must answer to is the identity.
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
