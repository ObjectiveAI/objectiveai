//! The agentic loop chunk — the unit of a streaming response.

use serde::{Deserialize, Serialize};

use super::{MessageChunk, Object, RemotePath, ResponseError, Upstream, Usage};

/// One chunk of a streaming agentic loop.
///
/// Chunks arrive continuously and accumulate via [`push`](Self::push)
/// into a single value that is the complete response. There is no
/// separate unary shape: the accumulation of the stream IS the result,
/// so a caller that wants the whole thing folds the stream rather than
/// asking for a different representation of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AgenticLoopChunk {
    /// This loop's id. Same on every chunk.
    pub id: String,
    /// The full lineage of the agent instance running this loop —
    /// ancestors plus the instance itself. Same on every chunk, and
    /// preserved verbatim across continuations, so an agent's identity
    /// survives being resumed by someone else.
    pub agent_instance_hierarchy: String,
    /// The id of the agent definition actually answering. On a
    /// fallback this is the fallback's id, not the primary's.
    pub agent_id: String,
    /// The id of the whole definition including its fallbacks. Unlike
    /// `agent_id` this does not change when a fallback takes over, so
    /// it identifies the request where `agent_id` identifies the
    /// responder.
    pub agent_full_id: String,
    /// Where the definition was fetched from. `None` when it was
    /// supplied inline and so has no remote to name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_remote: Option<RemotePath>,
    /// Unix seconds when the loop began. Same on every chunk.
    pub created: u64,
    /// The messages this chunk carries or extends.
    pub messages: Vec<MessageChunk>,
    /// Always [`Object::AgenticLoopChunk`].
    pub object: Object,
    /// Aggregate usage. Terminal chunk only — a loop's totals are not
    /// final until it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    /// Which upstream produced this.
    pub upstream: Upstream,
    /// A failure. Carried in-band so a loop that fails partway still
    /// delivers what it produced along with the reason it stopped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
    /// Opaque state for resuming. Terminal chunk only. Pass it back to
    /// continue the same conversation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
}

impl AgenticLoopChunk {
    /// Accumulate another chunk into this one.
    ///
    /// Only the fields that vary are folded. The identity fields —
    /// `id`, the three agent ids, `created`, `object`, `upstream` —
    /// are constant across a loop's chunks, so re-assigning them would
    /// be work that can only introduce a discrepancy.
    ///
    /// Messages merge by [`MessageChunk::index`], not by position: a
    /// turn streams across many chunks, and appending would leave the
    /// caller holding fragments instead of messages.
    ///
    /// `usage` sums rather than replaces, so folding a stream that
    /// reports usage more than once still totals correctly. `error`
    /// and `continuation` are last-wins.
    pub fn push(&mut self, other: &AgenticLoopChunk) {
        self.push_messages(&other.messages);
        match (&mut self.usage, &other.usage) {
            (Some(this), Some(that)) => this.push(that),
            (None, Some(that)) => self.usage = Some(that.clone()),
            _ => {}
        }
        if let Some(error) = &other.error {
            self.error = Some(error.clone());
        }
        if let Some(continuation) = &other.continuation {
            self.continuation = Some(continuation.clone());
        }
    }

    /// Merge messages by index, appending only those not seen yet.
    fn push_messages(&mut self, others: &[MessageChunk]) {
        for other in others {
            match self
                .messages
                .iter_mut()
                .find(|m| m.index() == other.index())
            {
                Some(message) => message.push(other),
                None => self.messages.push(other.clone()),
            }
        }
    }
}
