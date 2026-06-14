//! Swarm definitions and validation.
//!
//! An **Swarm** is a collection of [`Agent`](crate::agent::Agent)s
//! used together. Swarms are the foundation of ObjectiveAI's multi-model approach.
//!
//! # Key Properties
//!
//! - **Immutable**: Any change produces a new Swarm ID
//! - **No weights**: Weights are execution-time parameters, not part of the Swarm
//! - **Content-addressed**: IDs are deterministically computed from the definition
//! - **Deduplicated**: Duplicate agents are merged with their counts summed
//! - **Bounded**: Total agent count must be between 1 and 128 (individual agents with
//!   `count: 0` are skipped, but the sum of all counts must be at least 1)
//!
//! # Example
//!
//! ```
//! use objectiveai_sdk::swarm::{InlineSwarmBase, InlineSwarm};
//! use objectiveai_sdk::agent::{InlineAgentBase, InlineAgentBaseWithFallbacks, InlineAgentBaseWithFallbacksOrRemote, InlineAgentBaseWithFallbacksOrRemoteWithCount};
//! use objectiveai_sdk::agent::openrouter;
//! use objectiveai_sdk::agent::completions::message::{Message, SystemMessage, SimpleContent};
//!
//! let swarm_base = InlineSwarmBase {
//!     agents: vec![
//!         // A simple GPT-4 configuration
//!         InlineAgentBaseWithFallbacksOrRemoteWithCount {
//!             count: 1,
//!             inner: InlineAgentBaseWithFallbacksOrRemote::AgentBase(InlineAgentBaseWithFallbacks {
//!                 inner: InlineAgentBase::Openrouter(openrouter::AgentBase {
//!                     model: "openai/gpt-4o".to_string(),
//!                     ..Default::default()
//!                 }),
//!                 fallbacks: None,
//!             }),
//!         },
//!         // Claude with a system prompt
//!         InlineAgentBaseWithFallbacksOrRemoteWithCount {
//!             count: 1,
//!             inner: InlineAgentBaseWithFallbacksOrRemote::AgentBase(InlineAgentBaseWithFallbacks {
//!                 inner: InlineAgentBase::Openrouter(openrouter::AgentBase {
//!                     model: "anthropic/claude-3.5-sonnet".to_string(),
//!                     output_mode: openrouter::OutputMode::JsonSchema,
//!                     prefix_messages: Some(vec![
//!                         Message::System(SystemMessage {
//!                             content: SimpleContent::Text("You are a careful evaluator.".to_string()),
//!                             name: None,
//!                         }),
//!                     ]),
//!                     ..Default::default()
//!                 }),
//!                 fallbacks: None,
//!             }),
//!         },
//!         // Gemini with lower temperature
//!         InlineAgentBaseWithFallbacksOrRemoteWithCount {
//!             count: 2, // Include 2 instances
//!             inner: InlineAgentBaseWithFallbacksOrRemote::AgentBase(InlineAgentBaseWithFallbacks {
//!                 inner: InlineAgentBase::Openrouter(openrouter::AgentBase {
//!                     model: "google/gemini-2.0-flash-001".to_string(),
//!                     output_mode: openrouter::OutputMode::ToolCall,
//!                     temperature: Some(0.3),
//!                     ..Default::default()
//!                 }),
//!                 fallbacks: None,
//!             }),
//!         },
//!     ],
//!     weights: None,
//! };
//!
//! let swarm: InlineSwarm = swarm_base.convert(None).unwrap();
//! println!("Swarm ID: {}", swarm.id);
//! ```

mod swarm;

pub use swarm::*;

#[cfg(test)]
mod swarm_tests;
