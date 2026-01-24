//! Response types from the OpenRouter API.
//!
//! These types represent the upstream response format from OpenRouter and
//! provide methods to transform them into the downstream ObjectiveAI format.

mod chat_completion_chunk;
mod usage;

pub use chat_completion_chunk::*;
pub use usage::*;
