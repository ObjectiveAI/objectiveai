//! Token usage.

use serde::Deserialize;

/// Token counts, as `turn.completed` carries them.
///
/// The source documents these as "during the turn", and its JSONL
/// processor fills them from the LAST TOTAL token usage the core
/// reported for the thread (`usage_from_last_total`): the counts are
/// the thread's cumulative usage, so a resumed thread's second turn
/// reports both turns. A reader that wants the turn's own usage
/// subtracts what the previous turn reported — the converter's job,
/// since the harness resumes one thread across runs. Every field is
/// zero when the core reported nothing.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct Usage {
    /// Input tokens.
    pub input_tokens: i64,
    /// Input tokens served from the prompt cache.
    pub cached_input_tokens: i64,
    /// Input tokens written to the prompt cache. Absent on older
    /// wires; the source defaults it.
    #[serde(default)]
    pub cache_write_input_tokens: i64,
    /// Output tokens.
    pub output_tokens: i64,
    /// Output tokens spent reasoning.
    pub reasoning_output_tokens: i64,
}
