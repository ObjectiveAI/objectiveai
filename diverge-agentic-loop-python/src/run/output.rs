//! The envelope's value, as the turn's chunks.

use diverge_provider_sdk::shared::containers::run_loop::response::AgenticLoopChunk;

use super::{Envelope, Error};

/// Read the turn out of the envelope.
///
/// `eval` first: anything but `null` is the value. Otherwise what the
/// script printed, trimmed, is read as JSON — a script may print its
/// chunks instead of returning them — and nothing printed is
/// [`NoOutput`](Error::NoOutput). The value must be a JSON array of
/// chunks, and every chunk must be one the script is allowed to say:
/// the six assistant kinds and notifications. A `user`, `usage` or
/// `tool_response` chunk anywhere in it is
/// [`Forbidden`](Error::Forbidden), the whole output refused — the
/// script may call tools, and only call them; the answers, the
/// delivered messages and the accounting are the container's to
/// say. An empty array is a valid turn that said nothing.
pub fn output(envelope: Envelope) -> Result<Vec<AgenticLoopChunk>, Error> {
    let value = if envelope.eval.is_null() {
        let printed = envelope.stdout.trim();
        if printed.is_empty() {
            return Err(Error::NoOutput);
        }
        serde_json::from_str(printed).map_err(Error::Printed)?
    } else {
        envelope.eval
    };

    let chunks: Vec<AgenticLoopChunk> =
        serde_path_to_error::deserialize(value).map_err(Error::Deserialize)?;

    for (index, chunk) in chunks.iter().enumerate() {
        if let Some(kind) = forbidden(chunk) {
            return Err(Error::Forbidden { index, kind });
        }
    }
    Ok(chunks)
}

/// The kind a chunk may not be, if it is one.
fn forbidden(chunk: &AgenticLoopChunk) -> Option<&'static str> {
    match chunk {
        AgenticLoopChunk::User(_) => Some("user"),
        AgenticLoopChunk::Usage(_) => Some("usage"),
        AgenticLoopChunk::ToolResponse(_) => Some("tool_response"),
        AgenticLoopChunk::AssistantReasoning(_)
        | AgenticLoopChunk::AssistantTextContent(_)
        | AgenticLoopChunk::AssistantImageContent(_)
        | AgenticLoopChunk::AssistantAudioContent(_)
        | AgenticLoopChunk::AssistantToolCall(_)
        | AgenticLoopChunk::AssistantRefusal(_)
        | AgenticLoopChunk::Notification(_) => None,
    }
}
