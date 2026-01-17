//! Model specification for chat completion requests.

use crate::ensemble_llm;
use serde::{Deserialize, Serialize};

/// The model to use for chat completion.
///
/// Can be either:
/// - An inline [`EnsembleLlmBase`](ensemble_llm::EnsembleLlmBase) configuration
/// - The ID of a previously used Ensemble LLM (22-character base62 string)
///
/// Since IDs are content-addressed, ObjectiveAI stores Ensemble LLM definitions
/// when they are successfully used. "Previously used" means the ID exists in
/// ObjectiveAI's database from any successful use by anyone.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Model {
    /// The content-addressed ID of an Ensemble LLM stored in ObjectiveAI's database.
    Id(String),
    /// An inline Ensemble LLM configuration.
    Provided(ensemble_llm::EnsembleLlmBase),
}
