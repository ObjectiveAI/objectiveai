//! Response types for chat completions.
//!
//! This module contains types for parsing chat completion responses:
//!
//! - [`unary`] - Non-streaming (single response) types
//! - [`streaming`] - Streaming (Server-Sent Events) types
//! - Common types: [`FinishReason`], [`Usage`], [`Role`], [`Logprobs`]

mod finish_reason;
mod image;
mod logprobs;
mod role;
pub mod streaming;
pub mod unary;
mod usage;
pub mod util;

pub use finish_reason::*;
pub use image::*;
pub use logprobs::*;
pub use role::*;
pub use usage::*;
