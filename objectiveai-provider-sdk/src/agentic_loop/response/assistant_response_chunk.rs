//! A model turn, streamed.

use serde::{Deserialize, Serialize};

use super::{
    AssistantRole, AssistantToolCallDelta, FinishReason, Logprobs, RichContent,
    UpstreamUsage, util,
};

/// One assistant turn, delivered as a run of deltas.
///
/// Every chunk of one turn repeats the same `index`; that is the join
/// key, and it is why several turns can stream interleaved without the
/// consumer having to track arrival order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AssistantResponseChunk {
    /// Always `assistant`.
    pub role: AssistantRole,
    /// Position of this turn within the loop. The join key for
    /// accumulation.
    pub index: u64,
    /// Unix seconds when the turn began.
    pub created: u64,
    /// The model that produced it.
    pub model: String,
    /// The upstream's own id for this turn, for correlating against
    /// provider-side logs.
    pub upstream_id: String,
    /// Reasoning text, CONCATENATED across deltas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Tool calls, accumulated by their own `index`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<AssistantToolCallDelta>>,
    /// Content, accumulated per [`RichContent::push`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<RichContent>,
    /// A refusal, CONCATENATED across deltas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// Why the turn stopped. Present once the turn is over; `None`
    /// while it is still streaming.
    pub finish_reason: Option<FinishReason>,
    /// Per-token probabilities, when requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Logprobs>,
    /// The provider's service tier for this turn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    /// The provider's backend fingerprint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    /// Which provider served the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// This turn's own usage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UpstreamUsage>,
}

impl AssistantResponseChunk {
    /// Accumulate another delta of the SAME turn into this one.
    ///
    /// Three different rules, by field, and the difference is the
    /// point: text fields concatenate, keyed collections merge by key,
    /// and one-shot metadata is taken from whichever delta carries it
    /// first. `finish_reason` is deliberately first-wins — a turn ends
    /// once, and a later delta must not overwrite why.
    pub fn push(&mut self, other: &AssistantResponseChunk) {
        util::push_option_string(&mut self.reasoning, &other.reasoning);
        self.push_tool_calls(&other.tool_calls);
        match (&mut self.content, &other.content) {
            (Some(this), Some(that)) => this.push(that),
            (None, Some(that)) => self.content = Some(that.clone()),
            _ => {}
        }
        util::push_option_string(&mut self.refusal, &other.refusal);
        if self.finish_reason.is_none() {
            self.finish_reason = other.finish_reason;
        }
        match (&mut self.logprobs, &other.logprobs) {
            (Some(this), Some(that)) => this.push(that),
            (None, Some(that)) => self.logprobs = Some(that.clone()),
            _ => {}
        }
        if self.upstream_id.is_empty() {
            self.upstream_id = other.upstream_id.clone();
        }
        if self.service_tier.is_none() {
            self.service_tier = other.service_tier.clone();
        }
        if self.system_fingerprint.is_none() {
            self.system_fingerprint = other.system_fingerprint.clone();
        }
        if self.provider.is_none() {
            self.provider = other.provider.clone();
        }
        match (&mut self.usage, &other.usage) {
            (Some(this), Some(that)) => this.push(that),
            (None, Some(that)) => self.usage = Some(that.clone()),
            _ => {}
        }
    }

    /// Merge tool-call deltas by `index` rather than appending. A
    /// tool call arrives across many deltas, so appending would
    /// produce N fragments of one call instead of one call.
    fn push_tool_calls(&mut self, other: &Option<Vec<AssistantToolCallDelta>>) {
        match (self.tool_calls.as_mut(), other) {
            (Some(these), Some(those)) => {
                for that in those {
                    match these.iter_mut().find(|t| t.index == that.index) {
                        Some(this) => this.push(that),
                        None => these.push(that.clone()),
                    }
                }
            }
            (None, Some(those)) => self.tool_calls = Some(those.clone()),
            _ => {}
        }
    }
}
