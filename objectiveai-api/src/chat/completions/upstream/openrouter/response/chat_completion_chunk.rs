//! Chat completion chunk from OpenRouter streaming responses.

use serde::{Deserialize, Serialize};

/// A streaming chat completion chunk from OpenRouter.
///
/// Contains partial response data that arrives incrementally during streaming.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionChunk {
    /// Unique identifier for this completion from OpenRouter.
    pub id: String,
    /// Completion choices containing the generated content.
    pub choices: Vec<objectiveai::chat::completions::response::streaming::Choice>,
    /// Unix timestamp when the completion was created.
    pub created: u64,
    /// The model that generated this completion.
    pub model: String,
    /// Object type indicator.
    pub object: objectiveai::chat::completions::response::streaming::Object,
    /// The service tier used for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    /// System fingerprint for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    /// Token usage statistics (typically in the final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<super::Usage>,
    /// The upstream provider that served this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

impl ChatCompletionChunk {
    /// Transforms this upstream chunk into the downstream ObjectiveAI format.
    ///
    /// Replaces the upstream ID and model with ObjectiveAI's values while preserving
    /// the original values in `upstream_id` and `upstream_model` fields.
    pub fn into_downstream(
        self,
        id: String,
        model: String,
        is_byok: bool,
        cost_multiplier: rust_decimal::Decimal,
    ) -> objectiveai::chat::completions::response::streaming::ChatCompletionChunk {
        objectiveai::chat::completions::response::streaming::ChatCompletionChunk {
            id,
            upstream_id: self.id,
            choices: self.choices,
            created: self.created,
            model,
            upstream_model: self.model,
            object: self.object,
            service_tier: self.service_tier,
            system_fingerprint: self.system_fingerprint,
            usage: self
                .usage
                .map(|usage| usage.into_downstream(is_byok, cost_multiplier)),
            provider: self.provider,
        }
    }

    /// Merges another chunk into this one.
    ///
    /// Used to accumulate streaming chunks into a complete response.
    pub fn push(
        &mut self,
        ChatCompletionChunk {
            choices,
            service_tier,
            system_fingerprint,
            usage,
            provider,
            ..
        }: &ChatCompletionChunk,
    ) {
        self.push_choices(choices);
        if self.service_tier.is_none() {
            self.service_tier = service_tier.clone();
        }
        if self.system_fingerprint.is_none() {
            self.system_fingerprint = system_fingerprint.clone();
        }
        match (&mut self.usage, usage) {
            (Some(self_usage), Some(other_usage)) => {
                self_usage.push(other_usage);
            }
            (None, Some(other_usage)) => {
                self.usage = Some(other_usage.clone());
            }
            _ => {}
        }
        if self.provider.is_none() {
            self.provider = provider.clone();
        }
    }

    /// Merges choices from another chunk, matching by index.
    fn push_choices(
        &mut self,
        other_choices: &[objectiveai::chat::completions::response::streaming::Choice],
    ) {
        fn push_choice(
            choices: &mut Vec<objectiveai::chat::completions::response::streaming::Choice>,
            other: &objectiveai::chat::completions::response::streaming::Choice,
        ) {
            fn find_choice(
                choices: &mut Vec<objectiveai::chat::completions::response::streaming::Choice>,
                index: u64,
            ) -> Option<&mut objectiveai::chat::completions::response::streaming::Choice>
            {
                for choice in choices {
                    if choice.index == index {
                        return Some(choice);
                    }
                }
                None
            }
            if let Some(choice) = find_choice(choices, other.index) {
                choice.push(other);
            } else {
                choices.push(other.clone());
            }
        }
        for other_choice in other_choices {
            push_choice(&mut self.choices, other_choice);
        }
    }
}
