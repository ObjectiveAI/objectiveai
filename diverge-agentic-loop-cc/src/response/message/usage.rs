//! What the API billed.

use serde::{Deserialize, Serialize};

/// Token counts for one API message.
///
/// The pinned SDK's `BetaUsage` stops at the four token counts, but
/// the EMITTERS do not: Claude Code's own message constructors and
/// accumulators read and write six more keys — server-side tool
/// spend, service tier, cache-writes-by-lifetime, inference
/// geography, per-iteration detail, and speed — and real API
/// messages reach stdout by verbatim spread, so whatever the API
/// said rides along. The six are optional AND nullable (the
/// synthetic constructors write explicit `null`s), so they carry
/// `default` without `skip_serializing_if`: absent parses, and an
/// empty value re-emits as `null` the way the emitters spell it.
///
/// The cache counts are nullable in the SDK — a model or tier that
/// does not report caching sends `null`, not `0`, and the difference
/// is kept.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens billed, not counting cache reads or writes.
    pub input_tokens: u64,
    /// Output tokens billed.
    pub output_tokens: u64,
    /// Tokens written to the prompt cache, if reported.
    pub cache_creation_input_tokens: Option<u64>,
    /// Tokens read from the prompt cache, if reported.
    pub cache_read_input_tokens: Option<u64>,
    /// Server-side tool spend.
    #[serde(default)]
    pub server_tool_use: Option<ServerToolUse>,
    /// The service tier that served it. A string, because the
    /// vocabulary is the API's.
    #[serde(default)]
    pub service_tier: Option<String>,
    /// Cache writes, by lifetime.
    #[serde(default)]
    pub cache_creation: Option<CacheCreation>,
    /// Where inference ran.
    #[serde(default)]
    pub inference_geo: Option<String>,
    /// Per-iteration detail: what each pass of an adaptive turn
    /// spent, message passes and compaction passes alike.
    #[serde(default)]
    pub iterations: Option<Vec<IterationUsage>>,
    /// The speed tier that served it.
    #[serde(default)]
    pub speed: Option<String>,
}

/// One iteration's usage, discriminated by its `type` literal —
/// `message` and `compaction` today, and anything newer preserved in
/// [`Other`](Self::Other) rather than failing the whole usage.
///
/// The two known kinds carry identical counts; the literal says
/// which kind of pass spent them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IterationUsage {
    /// A model pass.
    Message {
        /// Always `message`.
        r#type: super::MessageType,
        /// Cache writes, by lifetime.
        cache_creation: Option<CacheCreation>,
        /// Tokens written to the prompt cache.
        #[serde(default)]
        cache_creation_input_tokens: u64,
        /// Tokens read from the prompt cache.
        #[serde(default)]
        cache_read_input_tokens: u64,
        /// Input tokens billed.
        #[serde(default)]
        input_tokens: u64,
        /// Output tokens billed.
        #[serde(default)]
        output_tokens: u64,
    },
    /// A compaction pass.
    Compaction {
        /// Always `compaction`.
        r#type: super::CompactionType,
        /// Cache writes, by lifetime.
        cache_creation: Option<CacheCreation>,
        /// Tokens written to the prompt cache.
        #[serde(default)]
        cache_creation_input_tokens: u64,
        /// Tokens read from the prompt cache.
        #[serde(default)]
        cache_read_input_tokens: u64,
        /// Input tokens billed.
        #[serde(default)]
        input_tokens: u64,
        /// Output tokens billed.
        #[serde(default)]
        output_tokens: u64,
    },
    /// An iteration newer than this crate, preserved verbatim.
    Other(serde_json::Value),
}

/// Server-side tool counts inside [`Usage`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
pub struct ServerToolUse {
    /// Web searches run.
    #[serde(default)]
    pub web_search_requests: u64,
    /// Web fetches run.
    #[serde(default)]
    pub web_fetch_requests: u64,
}

/// Cache writes by lifetime inside [`Usage`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
pub struct CacheCreation {
    /// Tokens cached for an hour.
    #[serde(default)]
    pub ephemeral_1h_input_tokens: u64,
    /// Tokens cached for five minutes.
    #[serde(default)]
    pub ephemeral_5m_input_tokens: u64,
}

/// `BetaMessageDeltaUsage`: the one cumulative count a
/// [`message_delta`](super::StreamEvent::MessageDelta) carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeltaUsage {
    /// The cumulative number of output tokens so far.
    pub output_tokens: u64,
}
