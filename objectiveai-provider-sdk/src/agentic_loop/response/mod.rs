//! Agentic loop response data.
//!
//! A response is a **stream** of [`AgenticLoopChunk`]s. Each carries
//! deltas of the messages the loop is producing; folding them with
//! [`AgenticLoopChunk::push`] yields the complete response.
//!
//! There is deliberately no separate non-streaming shape. A unary
//! response would be a second definition of the same thing, and two
//! definitions of one thing eventually disagree. The accumulation of
//! the stream IS the result.
//!
//! Mirrors the existing agent-completions response types. Expect it to
//! diverge as the loop is defined.

mod assistant_response_chunk;
mod chunk;
mod content;
mod error;
mod finish_reason;
mod logprobs;
mod message_chunk;
mod remote;
mod role;
mod tool_call;
mod tool_response;
mod upstream;
mod upstream_duration_ms;
mod usage;
mod usage_details;
pub mod util;

pub use assistant_response_chunk::*;
pub use chunk::*;
pub use content::*;
pub use error::*;
pub use finish_reason::*;
pub use logprobs::*;
pub use message_chunk::*;
pub use remote::*;
pub use role::*;
pub use tool_call::*;
pub use tool_response::*;
pub use upstream::*;
pub use upstream_duration_ms::*;
pub use usage::*;
pub use usage_details::*;
