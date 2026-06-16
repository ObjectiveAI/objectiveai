use serde::{Deserialize, Serialize};

use super::StdioEndStatus;

/// One line emitted by the runner on stdout. Always carries an `id`.
///
/// `T` is the per-event payload type — the [`super::super::GeminiEvent`]
/// type the gemini client deserializes its inner `event` into. `T` is
/// only touched on the [`StdioOutput::Event`] variant;
/// [`StdioOutput::End`] is `T`-free.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StdioOutput<T> {
    /// One inner event for an in-flight request.
    Event { id: String, event: T },
    /// Terminal line for a request — emitted exactly once per accepted
    /// `run`. The `status` discriminator is flattened into the outer
    /// object on the wire (see [`StdioEndStatus`]).
    End {
        id: String,
        #[serde(flatten)]
        status: StdioEndStatus,
    },
}
