//! What an OpenRouter continuation holds.

use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::AgenticLoopChunk;

/// A continuation, opened.
///
/// Opaque to everyone but this container: what it holds is the
/// conversation's own history — each turn's user prompt and the
/// chunks the loop produced, in order — as a JSON array. On the wire
/// it is those bytes, uncoated: no base64, no envelope. Resuming is
/// deserializing it and rebuilding the conversation from what was
/// already said.
#[derive(Debug, Clone, PartialEq)]
pub struct Continuation(pub Vec<ContinuationItem>);

/// One entry in the history.
///
/// Untagged, and unambiguous without a tag: a chunk serializes as a
/// JSON object — its `type` member inside — and a prompt as a JSON
/// string. An object and a string cannot collide.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ContinuationItem {
    /// One chunk the loop produced.
    Chunk(AgenticLoopChunk),
    /// One turn's user prompt, as its text.
    Prompt(String),
}

impl Continuation {
    /// Open the chunks the server delivered: joined, they are the
    /// history as JSON. The protocol keeps the chunks apart for
    /// containers that put meaning in the boundaries; this one does
    /// not — its closer is one document split at the chunk ceiling,
    /// and joining is the whole of reading it back.
    pub fn parse(chunks: &[Vec<u8>]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(&chunks.concat()).map(Continuation)
    }

    /// The history as the bytes the run closes with —
    /// [`parse`](Self::parse)'s exact inverse.
    pub fn tokenize(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self.0)
    }
}
