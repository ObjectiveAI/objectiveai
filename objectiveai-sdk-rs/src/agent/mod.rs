//! Agent definitions, configuration, and completion API types.
//!
//! An **Agent** is a fully-specified configuration of a single upstream
//! language model. It encapsulates:
//!
//! - Model identity (which LLM to use)
//! - Prompt structure (prefix/suffix messages)
//! - Decoding parameters (temperature, top_p, etc.)
//! - Provider preferences and routing
//! - Output mode, reasoning settings, and verbosity
//!
//! # Content-Addressed Identity
//!
//! Agents use **content-addressed identifiers** - their ID is derived
//! deterministically from their full definition using XXHash3-128. This ensures:
//!
//! - Two identical definitions always produce the same ID
//! - IDs can be computed anywhere (server, client, browser via WASM)
//! - No hidden mutation or "latest version" ambiguity
//!
//! # Normalization
//!
//! Before computing an ID, definitions are normalized via [`InlineAgentBase::prepare`]:
//!
//! - Default values are removed (e.g., `temperature: 1.0` becomes `None`)
//! - Empty collections are removed
//! - Collections are sorted for deterministic ordering

mod agent;
pub mod claude_agent_sdk;
mod client_objectiveai_mcp;
pub mod codex_sdk;
pub mod completions;
mod continuation;
mod laboratory;
mod mcp;
pub mod mock;
pub mod openrouter;
pub mod script;
mod output_mode;
mod upstream;

pub use agent::*;
pub use client_objectiveai_mcp::*;
pub use continuation::*;
pub use laboratory::*;
pub use mcp::*;
pub use output_mode::*;
pub use upstream::*;
