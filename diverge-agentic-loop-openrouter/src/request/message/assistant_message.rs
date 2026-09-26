//! Assistant messages and their tool calls.

use std::collections::HashMap;

use diverge_provider_sdk::endpoints::containers::agents::run::server::response;
use diverge_provider_sdk::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use serde::{Deserialize, Serialize};

use super::super::{RichContent, RichContentPart};

/// An assistant message (model's previous response).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
)]
pub struct AssistantMessage {
    /// The message content, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<RichContent>,
    /// Refusal message if the model declined to respond.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<AssistantToolCall>>,
    /// Reasoning content from models that support chain-of-thought.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

/// A tool call made by the assistant.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantToolCall {
    /// A function call with an ID and function details.
    Function {
        /// The unique ID of this tool call.
        id: String,
        /// The function being called.
        function: AssistantToolCallFunction,
    },
}

/// Details of a function call made by the assistant.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
)]
pub struct AssistantToolCallFunction {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass to the function, as a JSON string.
    pub arguments: String,
}

/// A tool call delta in a streaming response.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
pub struct AssistantToolCallDelta {
    /// The index of this tool call.
    pub index: u64,
    /// The type of tool call (always "function").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<AssistantToolCallType>,
    /// The unique ID of this tool call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The function call details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<AssistantToolCallFunctionDelta>,
}

/// The type of tool call.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
)]
pub enum AssistantToolCallType {
    /// A function call.
    #[serde(rename = "function")]
    #[default]
    Function,
}

/// Function call details in a streaming tool call.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
)]
pub struct AssistantToolCallFunctionDelta {
    /// The function name (only present in the first delta).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The arguments being streamed (accumulated across deltas).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

impl AssistantMessage {
    /// Start a message from its first chunk.
    ///
    /// The chunk must be one of the assistant kinds — the caller
    /// controls what arrives here, so anything else is unreachable.
    pub fn new(chunk: AgenticLoopChunk) -> Self {
        let mut message = AssistantMessage {
            content: None,
            refusal: None,
            tool_calls: None,
            reasoning: None,
        };
        message.push(chunk);
        message
    }

    /// Merge one more assistant chunk into this message.
    ///
    /// Content is kept as parts, in arrival order; reasoning and
    /// refusal text concatenate; tool calls accumulate. A chunk's log
    /// probabilities have no home in a message and are dropped. Only
    /// the assistant kinds arrive here — the caller controls that —
    /// so the rest are unreachable.
    pub fn push(&mut self, chunk: AgenticLoopChunk) {
        match chunk {
            AgenticLoopChunk::AssistantReasoning(chunk) => {
                append(&mut self.reasoning, chunk.inner.text);
            }
            AgenticLoopChunk::AssistantTextContent(chunk) => {
                self.push_part(RichContentPart::Text {
                    text: chunk.inner.text,
                });
            }
            AgenticLoopChunk::AssistantImageContent(chunk) => {
                self.push_part(chunk.inner.into());
            }
            AgenticLoopChunk::AssistantAudioContent(chunk) => {
                self.push_part(chunk.inner.into());
            }
            AgenticLoopChunk::AssistantToolCall(chunk) => {
                let arguments =
                    chunk.arguments.unwrap_or_else(|| String::from("{}"));
                self.tool_calls.get_or_insert_with(Vec::new).push(
                    super::AssistantToolCall::Function {
                        id: chunk.id,
                        function: super::AssistantToolCallFunction {
                            name: chunk.name,
                            arguments,
                        },
                    },
                );
            }
            AgenticLoopChunk::AssistantRefusal(chunk) => {
                append(&mut self.refusal, chunk.inner.text);
            }
            AgenticLoopChunk::ToolResponse(_)
            | AgenticLoopChunk::UserTextContent(_)
            | AgenticLoopChunk::UserImageContent(_)
            | AgenticLoopChunk::UserAudioContent(_)
            | AgenticLoopChunk::UserResource(_)
            | AgenticLoopChunk::UserResourceLink(_)
            | AgenticLoopChunk::Usage(_)
            | AgenticLoopChunk::Notification(_) => {
                unreachable!("only assistant chunks are pushed here")
            }
        }
    }

    /// Append one part, keeping `content` as parts. A plain-text
    /// content — which this type's own constructors never produce —
    /// is first rewrapped as a text part, so nothing is lost.
    fn push_part(&mut self, part: RichContentPart) {
        match &mut self.content {
            Some(RichContent::Parts(parts)) => parts.push(part),
            Some(RichContent::Text(_)) => {
                let Some(RichContent::Text(text)) = self.content.take()
                else {
                    unreachable!("matched Text above");
                };
                self.content = Some(RichContent::Parts(vec![
                    RichContentPart::Text { text },
                    part,
                ]));
            }
            None => {
                self.content = Some(RichContent::Parts(vec![part]));
            }
        }
    }
}

/// Concatenate streamed text into an optional accumulator.
fn append(accumulator: &mut Option<String>, text: String) {
    match accumulator {
        Some(existing) => existing.push_str(&text),
        None => *accumulator = Some(text),
    }
}

impl AssistantToolCallDelta {
    /// Append this fragment as a tool call chunk — when it can be
    /// named.
    ///
    /// OpenRouter sends a call's `id` and `name` only in its first
    /// fragment. A fragment that carries an id registers `(id, name)`
    /// in the map under its `index`; one that does not reads the map;
    /// an index the map cannot name is dropped, because a chunk
    /// without an id correlates with nothing.
    pub fn into_chunks(
        self,
        tool_calls: &mut HashMap<u64, (String, String)>,
        chunks: &mut Vec<response::AgenticLoopChunk>,
    ) {
        let name = self
            .function
            .as_ref()
            .and_then(|function| function.name.clone());
        let (id, name) = match self.id {
            Some(id) => {
                let entry = (id, name.unwrap_or_default());
                tool_calls.insert(self.index, entry.clone());
                entry
            }
            None => match tool_calls.get(&self.index) {
                Some(entry) => entry.clone(),
                None => return,
            },
        };
        chunks.push(response::AgenticLoopChunk::AssistantToolCall(
            response::AssistantToolCallChunk {
                r#type: Default::default(),
                parent_tool_call_id: None,
                id,
                meta: None,
                name,
                arguments: self.function.and_then(|function| function.arguments),
            },
        ));
    }
}
