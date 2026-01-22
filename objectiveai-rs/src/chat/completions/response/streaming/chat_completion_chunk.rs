//! Streaming chat completion chunk type.

use crate::chat::completions::response;
use serde::{Deserialize, Serialize};

/// A chunk of a streaming chat completion response.
///
/// Multiple chunks are received via Server-Sent Events and can be
/// accumulated into a complete [`ChatCompletion`](response::unary::ChatCompletion)
/// using the [`push`](Self::push) method.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionChunk {
    /// ObjectiveAI's unique identifier for this completion.
    pub id: String,
    /// The upstream provider's identifier.
    pub upstream_id: String,
    /// The choice deltas in this chunk.
    pub choices: Vec<super::Choice>,
    /// Unix timestamp when the completion was created.
    pub created: u64,
    /// The Ensemble LLM ID used.
    pub model: String,
    /// The upstream model identifier.
    pub upstream_model: String,
    /// The object type (always "chat.completion.chunk").
    pub object: super::Object,
    /// The service tier used, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    /// A fingerprint of the model configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    /// Token usage (only present in the final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<response::Usage>,

    /// The provider that served the request (OpenRouter-specific).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

impl ChatCompletionChunk {
    /// Accumulates another chunk into this one.
    ///
    /// This is used to build up a complete response from streaming chunks.
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

    fn push_choices(&mut self, other_choices: &[super::Choice]) {
        fn push_choice(
            choices: &mut Vec<super::Choice>,
            other: &super::Choice,
        ) {
            fn find_choice(
                choices: &mut Vec<super::Choice>,
                index: u64,
            ) -> Option<&mut super::Choice> {
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
