//! Chat completion API types.
//!
//! This module provides types for the ObjectiveAI chat completions API.
//! While inspired by the OpenAI chat completions format, it diverges in
//! several ways - notably, the `model` field must be an Ensemble LLM
//! configuration rather than a simple model string.

pub mod completions;
