//! Response types for agent completions.
//!
//! This module contains types for parsing agent completion responses:
//!
//! - [`unary`] - Non-streaming (single response) types
//! - [`streaming`] - Streaming (Server-Sent Events) types
//! - Common types: [`FinishReason`], [`Usage`], [`UpstreamUsage`], [`Role`], [`Logprobs`]

mod assistant_response;
mod finish_reason;
mod logprobs;
pub mod streaming;
mod tool_response;
#[cfg(feature = "filesystem")]
mod tool_response_log;
pub mod unary;
mod upstream_usage;
mod usage;
mod usage_details;
pub mod util;

pub use assistant_response::*;
pub use finish_reason::*;
pub use logprobs::*;
pub use tool_response::*;
#[cfg(feature = "filesystem")]
pub use tool_response_log::*;
pub use upstream_usage::*;
pub use usage::*;
pub use usage_details::*;
