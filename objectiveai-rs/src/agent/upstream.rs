//! Upstream enumeration.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Supported agent upstreams.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, JsonSchema, arbitrary::Arbitrary,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "agent.Upstream")]
pub enum Upstream {
    /// Unknown Upstream.
    #[schemars(title = "Unknown")]
    #[default]
    Unknown,
    /// OpenRouter Upstream.
    #[schemars(title = "Openrouter")]
    Openrouter,
    /// Claude Agent SDK Upstream.
    #[schemars(title = "ClaudeAgentSdk")]
    ClaudeAgentSdk,
    /// Mock Upstream.
    #[schemars(title = "Mock")]
    Mock,
}

pub mod validate {
    use super::Upstream;

    pub fn validate(upstreams: &[Upstream]) -> Result<(), String> {
        // Check for duplicates
        let mut seen = std::collections::HashSet::new();
        for upstream in upstreams {
            if !seen.insert(upstream) {
                return Err(
                    "`upstreams` contains duplicate entries".to_string()
                );
            }
        }

        // Check for Unknown
        if seen.contains(&Upstream::Unknown) {
            return Err("`upstreams` contains unknown upstream".to_string());
        }

        Ok(())
    }
}
