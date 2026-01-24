//! Vector completion operations.
//!
//! Vector completions produce scores rather than text. Multiple LLMs vote on
//! response options, and their votes are combined using weights to produce
//! final scores.

/// Vector completion client and types.
pub mod completions;
