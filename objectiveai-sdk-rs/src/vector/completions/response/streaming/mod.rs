//! Streaming response types for vector completions.
//!
//! - [`VectorCompletionChunk`] - Top-level streaming chunk
//! - [`AgentCompletionChunk`] - Individual agent completion chunk
//! - [`Object`] - Type marker (`"vector.completion.chunk"`)

mod agent_completion_chunk;
mod inner_error;
mod log_rows;
mod object;
mod vector_completion_chunk;

pub use agent_completion_chunk::*;
pub use inner_error::*;
pub use log_rows::*;
pub use object::*;
pub use vector_completion_chunk::*;

#[cfg(test)]
mod vector_completion_chunk_tests;
