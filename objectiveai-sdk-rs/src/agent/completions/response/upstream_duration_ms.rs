//! Per-upstream wall-clock duration, in milliseconds.

use super::util;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wall-clock duration spent inside each upstream client, in
/// milliseconds. Each upstream measures its own create→finish elapsed
/// per call and stamps its field on its terminal usage chunk; the
/// aggregators sum per-field across turns, fallbacks, and (at the
/// vector/function layers) across parallel agents. A field is `None`
/// when that upstream was never used.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.response.UpstreamDurationMs")]
pub struct UpstreamDurationMs {
    /// Milliseconds spent in the OpenRouter upstream.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub openrouter: Option<u64>,
    /// Milliseconds spent in the Claude Agent SDK upstream.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub claude_agent_sdk: Option<u64>,
    /// Milliseconds spent in the Codex SDK upstream.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub codex_sdk: Option<u64>,
}

impl UpstreamDurationMs {
    /// Returns `true` if any upstream recorded a duration. Presence is
    /// the signal (a sub-millisecond run legitimately measures 0).
    pub fn any_usage(&self) -> bool {
        self.openrouter.is_some()
            || self.claude_agent_sdk.is_some()
            || self.codex_sdk.is_some()
    }

    /// Appends durations from another instance, summing per-field.
    pub fn push(&mut self, other: &UpstreamDurationMs) {
        util::push_option_u64(&mut self.openrouter, &other.openrouter);
        util::push_option_u64(
            &mut self.claude_agent_sdk,
            &other.claude_agent_sdk,
        );
        util::push_option_u64(&mut self.codex_sdk, &other.codex_sdk);
    }
}
