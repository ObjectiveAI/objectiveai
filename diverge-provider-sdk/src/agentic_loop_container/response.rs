//! What a container streams back.

use serde::{Deserialize, Serialize};

use crate::endpoints::agentic_loop::run::server::response::AgenticLoopChunk;

/// One event of a container's response stream.
///
/// **Untagged, discriminated by payload**, the way this crate's JSON
/// unions are: an [`AgenticLoopChunk`] pins one of the chunk
/// vocabulary's own `type` constants, [`FetchResource`] pins
/// `fetch_resource` and [`FetchContinuation`] pins
/// `fetch_continuation`, neither of which is one of them.
///
/// Almost everything a container streams is the protocol's own
/// chunk vocabulary, one chunk per server-sent event, relayed to
/// the client unchanged. The other two variants are the exception
/// that made this a union: ASKS that ride the only stream the
/// container has, addressed to the SERVER rather than the client —
/// for a resource's bytes, or for the continuation the run resumes
/// from. See [`FetchResource`] and [`FetchContinuation`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    /// One chunk of the loop — the client's to receive, relayed
    /// verbatim. See
    /// [`AgenticLoopChunk`].
    Chunk(AgenticLoopChunk),
    /// The container asking for a resource's bytes. The server's to
    /// consume, never the client's to see. See [`FetchResource`].
    FetchResource(FetchResource),
    /// The container asking for the continuation it resumes from.
    /// The server's to consume, never the client's to see. See
    /// [`FetchContinuation`].
    FetchContinuation(FetchContinuation),
}

/// The container asking the server for a resource it only knows by
/// identity.
///
/// A request names resources as size-bearing identities — the FILE
/// grammar, `f1:<size>:<base64url sha256 of the bytes>` — and the
/// bytes live with the client, behind the server. When the
/// container needs them, this event is the ask: the server fetches
/// the content (its own
/// [`FetchResource`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchResource)
/// exchange toward the client, or its own store), then POSTs the
/// bytes into the container at `/resource/{identity}` — chunks,
/// then the completion, or the error when the bytes can never
/// come; see [`resource`](super::resource).
///
/// It is not a chunk and never reaches the client: the client is
/// what the bytes come FROM. A server relaying the stream forwards
/// [`Chunk`](Response::Chunk) events and consumes these.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FetchResource {
    /// Always `fetch_resource`.
    pub r#type: FetchResourceType,
    /// The resource's size-bearing identity, the FILE grammar:
    /// `f1:<size>:<base64url sha256 of the bytes>`.
    pub identity: String,
}

/// The `fetch_resource` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FetchResourceType {
    /// The only value.
    #[default]
    FetchResource,
}

/// The container asking the server for the continuation it resumes
/// from — [`FetchResource`]'s sibling with nothing to name, because
/// a run resumes from the one continuation its caller holds.
///
/// The container asks once, as the run starts and before it can
/// start the agent: it cannot know whether this is a first run or a
/// resume until the answer says. The server fetches the bytes from
/// the client over its own
/// [`FetchContinuation`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchContinuation)
/// exchange, then POSTs them into the container at `/continuation`
/// — chunks, then the completion (a LONE completion meaning a fresh
/// start), or the error when the bytes can never come; see
/// [`continuation`](super::continuation).
///
/// It is not a chunk and never reaches the client: the client is
/// what the bytes come FROM. A server relaying the stream forwards
/// [`Chunk`](Response::Chunk) events and consumes these.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FetchContinuation {
    /// Always `fetch_continuation`.
    pub r#type: FetchContinuationType,
}

/// The `fetch_continuation` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FetchContinuationType {
    /// The only value.
    #[default]
    FetchContinuation,
}
