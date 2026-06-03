//! Agent completion chunk from OpenRouter streaming responses.

use serde::{Deserialize, Serialize};

/// A streaming chat completion chunk from OpenRouter.
///
/// Contains partial response data that arrives incrementally during streaming.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionChunk {
    /// Unique identifier for this completion from OpenRouter.
    pub id: String,
    /// Completion choices containing the generated content.
    pub choices: Vec<super::Choice>,
    /// Unix timestamp when the completion was created.
    pub created: u64,
    /// The model that generated this completion.
    pub model: String,
    /// Object type indicator.
    pub object: super::Object,
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
    /// Transforms this upstream OpenRouter chunk into the downstream
    /// [`AgentCompletionChunk`] format.
    ///
    /// OpenRouter completions always have exactly one choice and only
    /// produce assistant responses (no tool responses).
    #[allow(clippy::too_many_arguments)]
    pub fn into_downstream(
        self,
        id: String,
        created: u64,
        index: u64,
        is_byok: bool,
        cost_multiplier: rust_decimal::Decimal,
        agent_instance_hierarchy: String,
        agent_id: String,
        agent_full_id: String,
        agent_remote: Option<objectiveai_sdk::RemotePath>,
    ) -> objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
        // OpenRouter always returns exactly one choice.
        let choice = self.choices.into_iter().next().unwrap_or_default();

        // Merge text content and images into a single RichContent value.
        let content = match (choice.delta.content, choice.delta.images) {
            (Some(text), Some(images)) => {
                let mut parts = vec![
                    objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                        text,
                    },
                ];
                parts.extend(images.into_iter().map(
                    objectiveai_sdk::agent::completions::message::RichContentPart::from,
                ));
                Some(objectiveai_sdk::agent::completions::message::RichContent::Parts(parts))
            }
            (Some(text), None) => {
                Some(objectiveai_sdk::agent::completions::message::RichContent::Text(text))
            }
            (None, Some(images)) => {
                Some(objectiveai_sdk::agent::completions::message::RichContent::Parts(
                    images
                        .into_iter()
                        .map(objectiveai_sdk::agent::completions::message::RichContentPart::from)
                        .collect(),
                ))
            }
            (None, None) => None,
        };

        let message = objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
            objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                role: Default::default(),
                index,
                created: self.created,
                model: self.model,
                upstream_id: self.id,
                reasoning: choice.delta.reasoning,
                tool_calls: choice.delta.tool_calls,
                content,
                refusal: choice.delta.refusal,
                finish_reason: choice.finish_reason,
                logprobs: choice.logprobs,
                service_tier: self.service_tier,
                system_fingerprint: self.system_fingerprint,
                provider: self.provider,
                usage: self
                    .usage
                    .map(|usage| usage.into_downstream(is_byok, cost_multiplier)),
            },
        );

        objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
            id,
            agent_instance_hierarchy,
            agent_id,
            agent_full_id,
            agent_remote,
            created,
            messages: vec![message],
            object: Default::default(),
            usage: None,
            upstream: objectiveai_sdk::agent::Upstream::Openrouter,
            error: None,
            continuation: None,
            messages_queued: None,
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
