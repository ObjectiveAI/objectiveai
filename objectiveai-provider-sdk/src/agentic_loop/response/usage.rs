//! Token and cost usage.

use serde::{Deserialize, Serialize};

use super::{
    CompletionTokensDetails, CostDetails, PromptTokensDetails,
    UpstreamDurationMs,
};

/// Token and cost usage.
///
/// One type at both scales: a single turn reports its own, and the
/// loop reports the sum of them across turns, tool rounds and
/// fallbacks. The loop-level total appears once, on the terminal
/// chunk, because it is not final until the loop is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Usage {
    /// Tokens generated, across every call.
    pub completion_tokens: u64,
    /// Prompt tokens consumed, across every call.
    pub prompt_tokens: u64,
    /// The two above, summed.
    pub total_tokens: u64,
    /// Where the completion tokens went.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
    /// Where the prompt tokens came from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    /// What ObjectiveAI charged.
    pub cost: rust_decimal::Decimal,
    /// Who charged what.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_details: Option<CostDetails>,
    /// Everything charged, including by upstream providers directly.
    ///
    /// Differs from `cost` only under BYOK, where the provider bills
    /// the caller rather than us — the money still moved, so it is
    /// still reported.
    pub total_cost: rust_decimal::Decimal,
    /// Wall-clock time per upstream.
    #[serde(default)]
    pub upstream_duration_ms: UpstreamDurationMs,
}

impl Usage {
    /// Whether anything at all was used.
    pub fn any_usage(&self) -> bool {
        self.completion_tokens > 0
            || self.prompt_tokens > 0
            || self.total_tokens > 0
            || self
                .completion_tokens_details
                .as_ref()
                .is_some_and(CompletionTokensDetails::any_usage)
            || self
                .prompt_tokens_details
                .as_ref()
                .is_some_and(PromptTokensDetails::any_usage)
            || self.cost > rust_decimal::Decimal::ZERO
            || self
                .cost_details
                .as_ref()
                .is_some_and(CostDetails::any_usage)
            || self.total_cost > rust_decimal::Decimal::ZERO
            || self.upstream_duration_ms.any_usage()
    }

    /// Sum another usage into this one.
    pub fn push(&mut self, other: &Usage) {
        self.completion_tokens += other.completion_tokens;
        self.prompt_tokens += other.prompt_tokens;
        self.total_tokens += other.total_tokens;
        push_details(
            &mut self.completion_tokens_details,
            &other.completion_tokens_details,
        );
        push_prompt_details(
            &mut self.prompt_tokens_details,
            &other.prompt_tokens_details,
        );
        self.cost += other.cost;
        push_cost_details(&mut self.cost_details, &other.cost_details);
        self.total_cost += other.total_cost;
        self.upstream_duration_ms.push(&other.upstream_duration_ms);
    }
}

fn push_details(
    slot: &mut Option<CompletionTokensDetails>,
    other: &Option<CompletionTokensDetails>,
) {
    match (slot.as_mut(), other) {
        (Some(this), Some(that)) => this.push(that),
        (None, Some(that)) => *slot = Some(that.clone()),
        _ => {}
    }
}

fn push_prompt_details(
    slot: &mut Option<PromptTokensDetails>,
    other: &Option<PromptTokensDetails>,
) {
    match (slot.as_mut(), other) {
        (Some(this), Some(that)) => this.push(that),
        (None, Some(that)) => *slot = Some(that.clone()),
        _ => {}
    }
}

fn push_cost_details(
    slot: &mut Option<CostDetails>,
    other: &Option<CostDetails>,
) {
    match (slot.as_mut(), other) {
        (Some(this), Some(that)) => this.push(that),
        (None, Some(that)) => *slot = Some(that.clone()),
        _ => {}
    }
}
