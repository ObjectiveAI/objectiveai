//! What a provider sends back on an agent container run: the id, the
//! agent's conversation, or a failure.
//!
//! [`Frame`] is what carries it; everything else here is what goes
//! inside a [`Chunk`](Frame::Chunk). The conversation is a **stream**
//! of [`AgenticLoopChunk`]s. Each chunk is one event — content,
//! reasoning, a tool call, a refusal, a tool result, usage, a
//! notification, or one block of a message of the caller's landing —
//! rather than a partially-filled record of everything that could
//! have happened.
//!
//! Content, tool calls and tool results are MCP's own types, flattened
//! — one content vocabulary across the whole conversation, so what a
//! model produces and what a tool returns need no translation between
//! them. The chunks are defined here, beside the frame that carries
//! them, because this stream is where they are read; the proxy's own
//! wire names them from here.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod assistant_audio_content_chunk;
mod assistant_image_content_chunk;
mod assistant_reasoning_chunk;
mod assistant_refusal_chunk;
mod assistant_text_content_chunk;
mod assistant_tool_call_chunk;
mod chunk;
mod frame;
mod logprobs;
mod notification_chunk;
mod push;
mod tool_response_chunk;
mod usage_chunk;
mod user_audio_content_chunk;
mod user_image_content_chunk;
mod user_parts;
mod user_resource_chunk;
mod user_resource_link_chunk;
mod user_text_content_chunk;

pub use assistant_audio_content_chunk::*;
pub use assistant_image_content_chunk::*;
pub use assistant_reasoning_chunk::*;
pub use assistant_refusal_chunk::*;
pub use assistant_text_content_chunk::*;
pub use assistant_tool_call_chunk::*;
pub use chunk::*;
pub use frame::*;
pub use logprobs::*;
pub use notification_chunk::*;
pub use push::*;
pub use tool_response_chunk::*;
pub use usage_chunk::*;
pub use user_audio_content_chunk::*;
pub use user_image_content_chunk::*;
pub use user_parts::*;
pub use user_resource_chunk::*;
pub use user_resource_link_chunk::*;
pub use user_text_content_chunk::*;
