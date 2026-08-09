//! Agentic loop response data.
//!
//! A response is a **stream** of [`AgenticLoopChunk`]s. Each chunk is
//! one event — content, reasoning, a tool call, a refusal, a tool
//! result, usage, an error, or the continuation — rather than a
//! partially-filled record of everything that could have happened.
//!
//! [`response_old`](super::response_old) holds the mirror of the
//! agent-completions types, kept alongside as the reference this
//! replaces rather than as something to migrate wholesale.

mod assistant_audio_content_chunk;
mod assistant_image_content_chunk;
mod assistant_text_content_chunk;
mod chunk;
mod content;
mod continuation_chunk;
mod error_chunk;
mod tool_response_chunk;
mod usage_chunk;

pub use assistant_audio_content_chunk::*;
pub use assistant_image_content_chunk::*;
pub use assistant_text_content_chunk::*;
pub use chunk::*;
pub use content::*;
pub use continuation_chunk::*;
pub use error_chunk::*;
pub use tool_response_chunk::*;
pub use usage_chunk::*;
