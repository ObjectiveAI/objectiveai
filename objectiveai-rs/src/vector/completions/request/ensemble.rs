//! Ensemble specification for vector completion requests.

use crate::ensemble;
use serde::{Deserialize, Serialize};

/// Specifies which Ensemble to use for a vector completion.
///
/// Ensembles can be referenced by ID or provided inline. The untagged
/// deserialization allows either a string ID or a full [`EnsembleBase`]
/// definition in JSON.
///
/// # Examples
///
/// By ID:
/// ```json
/// "ensemble": "ens_abc123"
/// ```
///
/// Inline definition:
/// ```json
/// "ensemble": {
///   "llms": [
///     {"model": "openai/gpt-4o", "output_mode": "json_schema", "count": 2},
///     {"model": "google/gemini-3.0-pro", "output_mode": "tool_call"}
///   ]
/// }
/// ```
///
/// [`EnsembleBase`]: crate::ensemble::EnsembleBase
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Ensemble {
    /// Reference an existing Ensemble by its ID.
    Id(String),
    /// Provide an inline Ensemble definition.
    Provided(ensemble::EnsembleBase),
}
