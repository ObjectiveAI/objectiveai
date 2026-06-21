//! Request types for OpenRouter API calls.
//!
//! This module transforms ObjectiveAI request types into the format expected
//! by the OpenRouter API, applying Agent configurations.

mod chat_completion_create_params;
mod message;
mod prediction;
mod provider;
mod response_format;
mod stream_options;
mod tool;
mod tool_choice;
mod usage;

pub use chat_completion_create_params::*;
pub use message::*;
pub use prediction::*;
pub use provider::*;
pub use response_format::*;
pub use stream_options::*;
pub use tool::*;
pub use tool_choice::*;
pub use usage::*;

#[cfg(test)]
mod chat_completion_create_params_tests;
