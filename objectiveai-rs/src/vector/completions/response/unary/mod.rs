//! Unary (non-streaming) response types for vector completions.
//!
//! - [`VectorCompletion`] - Complete vector completion response
//! - [`ChatCompletion`] - Individual LLM completion
//! - [`Object`] - Type marker (`"vector.completion"`)

mod chat_completion;
mod object;
mod vector_completion;

pub use chat_completion::*;
pub use object::*;
pub use vector_completion::*;
