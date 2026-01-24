//! Ensemble definitions and validation.
//!
//! An **Ensemble** is a collection of [`EnsembleLlm`](crate::ensemble_llm::EnsembleLlm)s
//! used together. Ensembles are the foundation of ObjectiveAI's multi-model approach.
//!
//! # Key Properties
//!
//! - **Immutable**: Any change produces a new Ensemble ID
//! - **No weights**: Weights are execution-time parameters, not part of the Ensemble
//! - **Content-addressed**: IDs are deterministically computed from the definition
//! - **Deduplicated**: Duplicate LLMs are merged with their counts summed
//! - **Bounded**: Total LLM count must be between 1 and 128 (individual LLMs with
//!   `count: 0` are skipped, but the sum of all counts must be at least 1)
//!
//! # Example
//!
//! ```
//! use objectiveai::ensemble::{EnsembleBase, Ensemble};
//! use objectiveai::ensemble_llm::{EnsembleLlmBase, EnsembleLlmBaseWithFallbacksAndCount, OutputMode};
//! use objectiveai::chat::completions::request::{Message, SystemMessage, SimpleContent};
//!
//! let ensemble_base = EnsembleBase {
//!     llms: vec![
//!         // A simple GPT-4 configuration
//!         EnsembleLlmBaseWithFallbacksAndCount {
//!             count: None,
//!             inner: EnsembleLlmBase {
//!                 model: "openai/gpt-4o".to_string(),
//!                 output_mode: OutputMode::Instruction,
//!                 ..Default::default()
//!             },
//!             fallbacks: None,
//!         },
//!         // Claude with a system prompt
//!         EnsembleLlmBaseWithFallbacksAndCount {
//!             count: None,
//!             inner: EnsembleLlmBase {
//!                 model: "anthropic/claude-3.5-sonnet".to_string(),
//!                 output_mode: OutputMode::JsonSchema,
//!                 prefix_messages: Some(vec![
//!                     Message::System(SystemMessage {
//!                         content: SimpleContent::Text("You are a careful evaluator.".to_string()),
//!                         name: None,
//!                     }),
//!                 ]),
//!                 ..Default::default()
//!             },
//!             fallbacks: None,
//!         },
//!         // Gemini with lower temperature
//!         EnsembleLlmBaseWithFallbacksAndCount {
//!             count: Some(2), // Include 2 instances
//!             inner: EnsembleLlmBase {
//!                 model: "google/gemini-2.0-flash-001".to_string(),
//!                 output_mode: OutputMode::ToolCall,
//!                 temperature: Some(0.3),
//!                 ..Default::default()
//!             },
//!             fallbacks: None,
//!         },
//!     ],
//! };
//!
//! let ensemble: Ensemble = ensemble_base.try_into().unwrap();
//! println!("Ensemble ID: {}", ensemble.id);
//! ```

mod ensemble;
pub mod response;

pub use ensemble::*;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "http")]
pub use http::*;
