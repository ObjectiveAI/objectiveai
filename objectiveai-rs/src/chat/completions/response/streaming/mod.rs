//! Streaming chat completion response types.
//!
//! These types are used when `stream: true`. Responses arrive as
//! Server-Sent Events (SSE), with each chunk containing a delta
//! of the full response.

mod chat_completion_chunk;
mod choice;
mod delta;
mod object;
mod tool_call;

pub use chat_completion_chunk::*;
pub use choice::*;
pub use delta::*;
pub use object::*;
pub use tool_call::*;
