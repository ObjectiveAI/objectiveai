//! What a server sends back on channel `0`.
//!
//! [`Frame`] is what carries it; everything else here is what goes
//! inside. A response is a **stream** of [`AgenticLoopChunk`]s. Each chunk is
//! one event — content, reasoning, a tool call, a refusal, a tool
//! result, usage, or a notification — rather than a partially-filled
//! record of everything that could have happened. The continuation
//! is not among them: it closes the stream as the frame's own
//! variant, raw bytes on a tag of their own.
//!
//! Content, tool calls and tool results are MCP's own types, flattened
//! — one content vocabulary across the whole loop, so what a model
//! produces and what a tool returns need no translation between them.

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
mod resource;
mod tool_response_chunk;
mod usage_chunk;
mod user_chunk;

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
pub use resource::*;
pub use tool_response_chunk::*;
pub use usage_chunk::*;
pub use user_chunk::*;
