//! Unary (non-streaming) response types for vector completions.
//!
//! - [`VectorCompletion`] - Complete vector completion response
//! - [`AgentCompletion`] - Individual agent completion
//! - [`Object`] - Type marker (`"vector.completion"`)

mod agent_completion;
mod object;
mod vector_completion;

pub use agent_completion::*;
pub use object::*;
pub use vector_completion::*;
