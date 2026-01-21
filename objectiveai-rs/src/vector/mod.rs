//! Vector completion API types.
//!
//! Vector completions produce **numbers** instead of text. Given a prompt and
//! a set of possible responses, vector completions:
//!
//! 1. Run multiple chat completions (one per LLM in the Ensemble)
//! 2. Force each completion to select one of the predefined responses
//! 3. Combine the selections using explicit weights (the "profile")
//! 4. Return a vector of scores that sums to 1
//!
//! Use vector completions for: picking a winner, ranking options, classification,
//! and producing machine-usable scoring outputs.

pub mod completions;
