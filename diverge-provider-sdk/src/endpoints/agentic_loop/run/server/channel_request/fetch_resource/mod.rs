//! Asking for one RESOURCE the provider is missing.
//!
//! A resource is arbitrary bytes with the FILE identity —
//! `f1:<size>:<base64url sha256 of the bytes>`, the same grammar
//! `file_mounts` uses, deliberately: a resource is bytes, so its
//! identity is a file's, and one content-addressed store can serve
//! both exchanges. The size rides the identity so a provider can
//! judge the weight before fetching anything.
//!
//! What names a resource is not a mount but an agent parameter — the
//! `*_resource` fields on provider structures (hermes's rotating
//! OAuth state) — and where a mount is immutable content placed in a
//! filesystem, a resource is STATE: the run mutates it, and its next
//! form is surfaced back to the caller. The fetch is the same
//! either way: the bytes live with the client, and a provider
//! missing them opens a channel with a [`Request`] naming the
//! identity. The client answers with the bytes themselves:
//! [`fetch_resource::Frame`](crate::endpoints::agentic_loop::run::client::channel_response::fetch_resource::Frame)s
//! of at most
//! [`CHUNK_SIZE`](crate::endpoints::agentic_loop::run::client::channel_response::CHUNK_SIZE)
//! each, every frame appending to the one resource, then the finish.
//!
//! # By identity, not by field
//!
//! The request carries no field name and no provider. A field is
//! where the CALLER put the resource and may name different content
//! tomorrow; the identity IS the content. The client indexes its
//! store however it likes — what it must answer to is the identity.
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
//! Nothing but an agentic loop fetches resources, so this lives
//! where it is used — a shape moves to `shared` when a second
//! endpoint needs it, not before.

mod request;

pub use request::*;
