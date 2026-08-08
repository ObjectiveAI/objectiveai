//! Wall-clock time spent in each upstream.

use serde::{Deserialize, Serialize};

use super::util;

/// Milliseconds spent inside each upstream, summed across turns and
/// fallbacks.
///
/// One field per upstream rather than a single total, because a loop
/// can cross upstreams — a fallback moves to a different one mid-run,
/// and attributing the whole duration to whichever answered last would
/// be wrong. `None` means that upstream was never entered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpstreamDurationMs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openrouter: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_agent_sdk: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codex_sdk: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<u64>,
}

impl UpstreamDurationMs {
    /// Whether any upstream recorded a duration.
    ///
    /// PRESENCE is the signal, not magnitude: a sub-millisecond run
    /// legitimately measures zero, so testing `> 0` would call it
    /// unused.
    pub fn any_usage(&self) -> bool {
        self.openrouter.is_some()
            || self.claude_agent_sdk.is_some()
            || self.codex_sdk.is_some()
            || self.script.is_some()
    }

    /// Sum another set of durations into this one, per upstream.
    pub fn push(&mut self, other: &UpstreamDurationMs) {
        util::push_option_u64(&mut self.openrouter, &other.openrouter);
        util::push_option_u64(
            &mut self.claude_agent_sdk,
            &other.claude_agent_sdk,
        );
        util::push_option_u64(&mut self.codex_sdk, &other.codex_sdk);
        util::push_option_u64(&mut self.script, &other.script);
    }
}
