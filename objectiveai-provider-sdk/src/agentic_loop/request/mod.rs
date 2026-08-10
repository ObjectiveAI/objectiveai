//! Agentic loop request data.
//!
//! What a caller hands a provider to start or resume a loop.
//!
//! The conversation is a list of [`Message`]s in the same vocabulary
//! the response emits — MCP content blocks, MCP tool calls, MCP tool
//! results — so feeding a loop's output back to it is a copy rather
//! than a translation.

mod assistant_message;
mod message;
mod request;
mod tool_call;
mod tool_message;
mod user_message;

pub use assistant_message::*;
pub use message::*;
pub use request::*;
pub use tool_call::*;
pub use tool_message::*;
pub use user_message::*;
