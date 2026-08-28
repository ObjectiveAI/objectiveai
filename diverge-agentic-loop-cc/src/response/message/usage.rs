//! What the API billed.

use serde::Deserialize;

/// Token counts for one API message.
///
/// The pinned SDK's `BetaUsage` stops at the four token counts, but
/// the EMITTERS do not: Claude Code's own message constructors and
/// accumulators read and write six more keys — server-side tool
/// spend, service tier, cache-writes-by-lifetime, inference
/// geography, per-iteration detail, and speed — and real API
/// messages reach stdout by verbatim spread, so whatever the API
/// said rides along. The six are optional AND nullable (the
/// synthetic constructors write explicit `null`s), so they are plain
/// `Option`s, both spellings landing as `None`. Every count that can
/// be absent is an `Option` too — absent and zero are different
/// facts, and a reader that invents zeros is editorializing.
///
/// The cache counts are nullable in the SDK — a model or tier that
/// does not report caching sends `null`, not `0`, and the difference
/// is kept.
#[derive(Debug, Clone, PartialEq, Deserialize)]
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
    pub server_tool_use: Option<ServerToolUse>,
    /// The service tier that served it. A string, because the
    /// vocabulary is the API's.
    pub service_tier: Option<String>,
    /// Cache writes, by lifetime.
    pub cache_creation: Option<CacheCreation>,
    /// Where inference ran.
    pub inference_geo: Option<String>,
    /// Per-iteration detail: what each pass of an adaptive turn
    /// spent, message passes and compaction passes alike.
    pub iterations: Option<Vec<IterationUsage>>,
    /// The speed tier that served it.
    pub speed: Option<String>,
}

/// One iteration's usage, discriminated by its `type` literal —
/// `message` and `compaction` today, and anything newer preserved in
/// [`Other`](Self::Other) rather than failing the whole usage.
///
/// The two known kinds carry identical counts; the literal says
/// which kind of pass spent them.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum IterationUsage {
    /// A model pass.
    Message {
        /// Always `message`.
        r#type: super::MessageType,
        /// Cache writes, by lifetime.
        cache_creation: Option<CacheCreation>,
        /// Tokens written to the prompt cache.
        cache_creation_input_tokens: Option<u64>,
        /// Tokens read from the prompt cache.
        cache_read_input_tokens: Option<u64>,
        /// Input tokens billed.
        input_tokens: Option<u64>,
        /// Output tokens billed.
        output_tokens: Option<u64>,
    },
    /// A compaction pass.
    Compaction {
        /// Always `compaction`.
        r#type: super::CompactionType,
        /// Cache writes, by lifetime.
        cache_creation: Option<CacheCreation>,
        /// Tokens written to the prompt cache.
        cache_creation_input_tokens: Option<u64>,
        /// Tokens read from the prompt cache.
        cache_read_input_tokens: Option<u64>,
        /// Input tokens billed.
        input_tokens: Option<u64>,
        /// Output tokens billed.
        output_tokens: Option<u64>,
    },
    /// An iteration newer than this crate, preserved verbatim.
    Other(serde_json::Value),
}

/// Server-side tool counts inside [`Usage`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default,
)]
pub struct ServerToolUse {
    /// Web searches run.
    pub web_search_requests: Option<u64>,
    /// Web fetches run.
    pub web_fetch_requests: Option<u64>,
}

/// Cache writes by lifetime inside [`Usage`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default,
)]
pub struct CacheCreation {
    /// Tokens cached for an hour.
    pub ephemeral_1h_input_tokens: Option<u64>,
    /// Tokens cached for five minutes.
    pub ephemeral_5m_input_tokens: Option<u64>,
}

/// The cumulative usage a
/// [`message_delta`](super::StreamEvent::MessageDelta) carries.
///
/// The pinned SDK knew only `output_tokens`; the current API sends
/// the whole running bill, everything but the output count nullable.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DeltaUsage {
    /// The cumulative number of output tokens so far.
    pub output_tokens: u64,
    /// Cumulative input tokens, when reported.
    pub input_tokens: Option<u64>,
    /// Cumulative cache writes, when reported.
    pub cache_creation_input_tokens: Option<u64>,
    /// Cumulative cache reads, when reported.
    pub cache_read_input_tokens: Option<u64>,
    /// Server-side tool spend so far, when reported.
    pub server_tool_use: Option<ServerToolUse>,
    /// Per-iteration detail so far, when reported.
    pub iterations: Option<Vec<IterationUsage>>,
}
