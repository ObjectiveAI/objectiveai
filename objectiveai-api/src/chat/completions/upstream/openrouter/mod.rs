//! OpenRouter provider client for LLM inference.
//!
//! This module provides the client implementation for communicating with the
//! OpenRouter API to perform chat completions.

mod client;
mod error;
/// Request types for OpenRouter API calls.
pub mod request;
/// Response types from OpenRouter API.
pub mod response;

pub use client::*;
pub use error::*;
