//! Request types for agent completions.
//!
//! - [`AgentCompletionCreateParams`] - The main request structure
//! - [`AgentCompletionNotifyParams`] - Inject a user message into a running completion
//! - [`Message`] - Agent messages (system, user, assistant, tool, developer)
//! - [`ResponseFormat`] - Output format constraints (text, JSON, JSON schema)
//! - [`Provider`] - Provider routing preferences

mod agent_completion_create_params;
#[cfg(feature = "filesystem")]
mod agent_completion_create_params_log;
mod agent_completion_notify_params;
mod provider;
mod response_format;

pub use agent_completion_create_params::*;
#[cfg(feature = "filesystem")]
pub use agent_completion_create_params_log::*;
pub use agent_completion_notify_params::*;
pub use provider::*;
pub use response_format::*;
