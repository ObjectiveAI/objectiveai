//! Tool messages.

use diverge_provider_sdk::shared::containers::run_loop::response::AgenticLoopChunk;
use serde::Serialize;

use super::super::{RichContent, RichContentPart};

/// A tool message containing the result of a tool call.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
)]
pub struct ToolMessage {
    /// The content of the tool response.
    pub content: RichContent,
    /// The ID of the tool call this message responds to.
    pub tool_call_id: String,
}

impl ToolMessage {
    /// A tool response chunk, as the message it is: the result's
    /// content blocks become parts, and the chunk's id names the call
    /// this answers. `is_error` and structured content have no
    /// message home and are dropped.
    ///
    /// Only `tool_response` chunks arrive here — the caller controls
    /// that — so every other kind is unreachable.
    pub fn new(chunk: AgenticLoopChunk) -> Self {
        let AgenticLoopChunk::ToolResponse(chunk) = chunk else {
            unreachable!("only tool response chunks are given here");
        };
        ToolMessage {
            content: RichContent::Parts(
                chunk.inner.content.into_iter().map(Into::into).collect(),
            ),
            tool_call_id: chunk.id,
        }
    }

    /// Fold steered messages into this tool response.
    ///
    /// Messages enqueued while the tools ran enter the conversation
    /// right behind the tool answers, and this is how the model
    /// sees them: one section spliced AHEAD of the tool's own
    /// content — the position the old proxy's queue notifications
    /// used — the texts joined by a blank line, the whole wrapped in
    /// a `system-reminder` pair. Tokenless, unlike the old form:
    /// delivery is confirmed by the container's own fates now, and
    /// nothing downstream scans for a token anymore.
    ///
    /// Derived here and ONLY here: the continuation stores the
    /// delivered prompts as bare `Prompt` items, and this section is
    /// what the request builder makes of them each time, not a thing
    /// the history remembers.
    pub fn fold_steer(&mut self, texts: &[String]) {
        let section = format!(
            "<system-reminder>\nThe user sent a new message while you were working:\n{}\n</system-reminder>\n\n",
            texts.join("\n\n"),
        );
        match &mut self.content {
            RichContent::Parts(parts) => {
                parts.insert(0, RichContentPart::Text { text: section });
            }
            RichContent::Text(text) => {
                *text = format!("{section}{text}");
            }
        }
    }
}
