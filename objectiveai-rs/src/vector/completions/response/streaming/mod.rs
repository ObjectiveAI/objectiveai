//! Streaming response types for vector completions.
//!
//! - [`VectorCompletionChunk`] - Top-level streaming chunk
//! - [`ChatCompletionChunk`] - Individual LLM completion chunk
//! - [`Object`] - Type marker (`"vector.completion.chunk"`)

mod chat_completion_chunk;
mod object;
mod vector_completion_chunk;

pub use chat_completion_chunk::*;
pub use object::*;
pub use vector_completion_chunk::*;
