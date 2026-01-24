//! Upstream provider enumeration.

use serde::{Deserialize, Serialize};

/// Supported upstream LLM providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Upstream {
    /// OpenRouter provider.
    OpenRouter,
}

/// Returns an iterator over available upstream providers for a request.
pub fn upstreams(
    _ensemble_llm: &objectiveai::ensemble_llm::EnsembleLlm,
    _request: super::Params,
) -> impl Iterator<Item = Upstream> {
    const ALL_UPSTREAMS: [Upstream; 1] = [Upstream::OpenRouter];
    ALL_UPSTREAMS.iter().copied()
}
