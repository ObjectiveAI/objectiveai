//! OpenRouter Agent types and validation logic.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use twox_hash::XxHash3_128;

/// The base configuration for an OpenRouter Agent (without computed ID).
#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.openrouter.AgentBase")]
pub struct AgentBase {
    /// The upstream provider marker.
    pub upstream: super::Upstream,

    /// The upstream language model identifier (e.g., `"gpt-4"`, `"claude-3-opus"`).
    pub model: String,

    /// The output mode for vector completions. Ignored for agent completions.
    #[serde(default)]
    pub output_mode: super::OutputMode,

    /// Enable synthetic reasoning for non-reasoning LLMs.
    ///
    /// **Vector completions only.** Ignored for agent completions.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub synthetic_reasoning: Option<bool>,

    /// Number of top log probabilities to return (2-20).
    ///
    /// **Vector completions only.** Ignored for agent completions.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub top_logprobs: Option<u64>,

    /// The agent's system prompt: a single role+content entry, rendered as the
    /// leading message of every request.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub system_prompt: Option<super::system_prompt::SystemPrompt>,

    /// Messages prepended to the user's prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub prefix_messages:
        Option<Vec<super::super::completions::message::Message>>,

    /// Messages appended after the user's prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub suffix_messages:
        Option<Vec<super::super::completions::message::Message>>,

    /// MCP servers the agent can connect to.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub mcp_servers: Option<super::super::McpServers>,

    /// Laboratories provisioned for the agent — each becomes a
    /// client-side laboratory MCP server whose id DERIVES from the
    /// agent's full id plus the spec (see
    /// [`laboratories::derived_id`](super::super::laboratory::laboratories::derived_id)).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub laboratories: Option<super::super::Laboratories>,

    /// Client-side ObjectiveAI MCP surface the calling client is
    /// expected to expose locally back to the API (objectiveai
    /// built-in, plus specific plugins / tools by owner+name+version).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub client_objectiveai_mcp: Option<super::super::ClientObjectiveaiMcp>,

    // --- OpenAI-compatible parameters ---
    /// Penalizes tokens based on their frequency in the output so far (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_f64)]
    pub frequency_penalty: Option<f64>,
    /// Token ID to bias mapping (-100 to 100). Positive values increase likelihood.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_indexmap_string_i64)]
    pub logit_bias: Option<IndexMap<String, i64>>,
    /// Maximum tokens in the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub max_completion_tokens: Option<u64>,
    /// Penalizes tokens based on their presence in the output so far (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_f64)]
    pub presence_penalty: Option<f64>,
    /// Stop sequences that halt generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stop: Option<super::Stop>,
    /// Sampling temperature (0.0 to 2.0). Higher = more random.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_f64)]
    pub temperature: Option<f64>,
    /// Nucleus sampling probability (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_f64)]
    pub top_p: Option<f64>,

    // --- OpenRouter-specific parameters ---
    /// Maximum tokens (OpenRouter variant of max_completion_tokens).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub max_tokens: Option<u64>,
    /// Minimum probability threshold for sampling (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_f64)]
    pub min_p: Option<f64>,
    /// Provider routing preferences.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<super::Provider>,
    /// Reasoning/thinking configuration for supported models.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<super::Reasoning>,
    /// Repetition penalty (0.0 to 2.0). Values > 1.0 penalize repetition.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_f64)]
    pub repetition_penalty: Option<f64>,
    /// Top-a sampling parameter (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_f64)]
    pub top_a: Option<f64>,
    /// Top-k sampling: only consider the k most likely tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub top_k: Option<u64>,
    /// Output verbosity hint for supported models.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub verbosity: Option<super::Verbosity>,
    /// Context compression engine for long contexts. When set, the
    /// upstream client emits the matching `plugins` entry on the
    /// outgoing OpenRouter chat-completions request.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub context_compression: Option<super::ContextCompression>,
}

impl AgentBase {
    /// Normalizes the configuration for deterministic ID computation.
    pub fn prepare(&mut self) {
        self.synthetic_reasoning = match self.synthetic_reasoning {
            Some(false) => None,
            other => other,
        };
        self.top_logprobs = match self.top_logprobs {
            Some(0) | Some(1) => None,
            other => other,
        };
        self.prefix_messages = match self.prefix_messages.take() {
            Some(prefix_messages) if prefix_messages.is_empty() => None,
            Some(mut prefix_messages) => {
                super::super::completions::message::prompt::prepare(
                    &mut prefix_messages,
                );
                if prefix_messages.is_empty() {
                    None
                } else {
                    Some(prefix_messages)
                }
            }
            None => None,
        };
        self.suffix_messages = match self.suffix_messages.take() {
            Some(suffix_messages) if suffix_messages.is_empty() => None,
            Some(mut suffix_messages) => {
                super::super::completions::message::prompt::prepare(
                    &mut suffix_messages,
                );
                if suffix_messages.is_empty() {
                    None
                } else {
                    Some(suffix_messages)
                }
            }
            None => None,
        };
        self.mcp_servers = match self.mcp_servers.take() {
            Some(mcp_servers) => {
                super::super::mcp::mcp_servers::prepare(mcp_servers)
            }
            None => None,
        };
        self.laboratories = match self.laboratories.take() {
            Some(laboratories) => {
                super::super::laboratory::laboratories::prepare(laboratories)
            }
            None => None,
        };
        self.client_objectiveai_mcp = match self.client_objectiveai_mcp.take() {
            Some(cm) => super::super::client_objectiveai_mcp::prepare(cm),
            None => None,
        };
        self.frequency_penalty = match self.frequency_penalty {
            Some(frequency_penalty) if frequency_penalty == 0.0 => None,
            other => other,
        };
        self.logit_bias = match self.logit_bias.take() {
            Some(logit_bias) if logit_bias.is_empty() => None,
            Some(mut logit_bias) => {
                logit_bias.retain(|_, &mut weight| weight != 0);
                logit_bias.sort_unstable_keys();
                Some(logit_bias)
            }
            None => None,
        };
        self.max_completion_tokens = match self.max_completion_tokens {
            Some(0) => None,
            other => other,
        };
        self.presence_penalty = match self.presence_penalty {
            Some(presence_penalty) if presence_penalty == 0.0 => None,
            other => other,
        };
        self.stop = match self.stop.take() {
            Some(stop) => stop.prepare(),
            None => None,
        };
        self.temperature = match self.temperature {
            Some(temperature) if temperature == 1.0 => None,
            other => other,
        };
        self.top_p = match self.top_p {
            Some(top_p) if top_p == 1.0 => None,
            other => other,
        };
        self.max_tokens = match self.max_tokens {
            Some(0) => None,
            other => other,
        };
        self.min_p = match self.min_p {
            Some(min_p) if min_p == 0.0 => None,
            other => other,
        };
        self.provider = match self.provider.take() {
            Some(provider) => provider.prepare(),
            None => None,
        };
        self.reasoning = match self.reasoning.take() {
            Some(reasoning) => reasoning.prepare(),
            None => None,
        };
        self.repetition_penalty = match self.repetition_penalty {
            Some(repetition_penalty) if repetition_penalty == 1.0 => None,
            other => other,
        };
        self.top_a = match self.top_a {
            Some(top_a) if top_a == 0.0 => None,
            other => other,
        };
        self.top_k = match self.top_k {
            Some(0) => None,
            other => other,
        };
        self.verbosity = match self.verbosity.take() {
            Some(verbosity) => verbosity.prepare(),
            None => None,
        };
        self.context_compression = match self.context_compression.take() {
            Some(cc) => cc.prepare(),
            None => None,
        };
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), String> {
        fn validate_f64(
            name: &str,
            value: Option<f64>,
            min: f64,
            max: f64,
        ) -> Result<(), String> {
            if let Some(v) = value {
                if !v.is_finite() {
                    return Err(format!("`{}` must be a finite number", name));
                }
                if v < min || v > max {
                    return Err(format!(
                        "`{}` must be between {} and {}",
                        name, min, max
                    ));
                }
            }
            Ok(())
        }
        fn validate_u64(
            name: &str,
            value: Option<u64>,
            min: u64,
            max: u64,
        ) -> Result<(), String> {
            if let Some(v) = value {
                if v < min || v > max {
                    return Err(format!(
                        "`{}` must be between {} and {}",
                        name, min, max
                    ));
                }
            }
            Ok(())
        }
        if self.model.is_empty() {
            return Err("`model` string cannot be empty".to_string());
        }
        if self.synthetic_reasoning.is_some()
            && let super::OutputMode::Instruction = self.output_mode
        {
            return Err(
                "`synthetic_reasoning` cannot be true when `output_mode` is \"instruction\""
                    .to_string(),
            );
        }
        if let Some(top_logprobs) = self.top_logprobs
            && top_logprobs > 20
        {
            return Err("`top_logprobs` must be at most 20".to_string());
        }
        if let Some(mcp_servers) = &self.mcp_servers {
            super::super::mcp::mcp_servers::validate(mcp_servers)?;
        }
        if let Some(laboratories) = &self.laboratories {
            super::super::laboratory::laboratories::validate(laboratories)?;
        }
        if let Some(cm) = &self.client_objectiveai_mcp {
            super::super::client_objectiveai_mcp::validate(cm)?;
        }
        validate_f64("frequency_penalty", self.frequency_penalty, -2.0, 2.0)?;
        if let Some(logit_bias) = &self.logit_bias {
            for (token, weight) in logit_bias {
                if token.is_empty() {
                    return Err("`logit_bias` keys cannot be empty".to_string());
                } else if !token.chars().all(|c| c.is_ascii_digit()) {
                    return Err(
                        "`logit_bias` keys must be stringified token IDs"
                            .to_string(),
                    );
                } else if token.chars().next().unwrap() == '0'
                    && token.len() > 1
                {
                    return Err("`logit_bias` keys cannot have leading zeros"
                        .to_string());
                } else if *weight < -100 || *weight > 100 {
                    return Err(
                        "`logit_bias` values must be between -100 and 100"
                            .to_string(),
                    );
                }
            }
        }
        validate_u64(
            "max_completion_tokens",
            self.max_completion_tokens,
            0,
            i32::MAX as u64,
        )?;
        validate_f64("presence_penalty", self.presence_penalty, -2.0, 2.0)?;
        if let Some(stop) = &self.stop {
            stop.validate()?;
        }
        validate_f64("temperature", self.temperature, 0.0, 2.0)?;
        validate_f64("top_p", self.top_p, 0.0, 1.0)?;
        validate_u64("max_tokens", self.max_tokens, 0, i32::MAX as u64)?;
        validate_f64("min_p", self.min_p, 0.0, 1.0)?;
        if let Some(provider) = &self.provider {
            provider.validate()?;
        }
        if let Some(reasoning) = &self.reasoning {
            reasoning.validate()?;
        }
        validate_f64("repetition_penalty", self.repetition_penalty, 0.0, 2.0)?;
        validate_f64("top_a", self.top_a, 0.0, 1.0)?;
        validate_u64("top_k", self.top_k, 0, i32::MAX as u64)?;
        if let Some(verbosity) = &self.verbosity {
            verbosity.validate()?;
        }
        if let Some(cc) = &self.context_compression {
            cc.validate()?;
        }
        if let Some(system_prompt) = &self.system_prompt {
            system_prompt.validate()?;
        }
        Ok(())
    }

    /// Returns prefix messages, then the provided messages, then suffix messages.
    pub fn merged_messages(
        &self,
        messages: Vec<super::super::completions::message::Message>,
    ) -> Vec<super::super::completions::message::Message> {
        let prefix_len = self.prefix_messages.as_ref().map_or(0, |m| m.len());
        let suffix_len = self.suffix_messages.as_ref().map_or(0, |m| m.len());
        let mut merged =
            Vec::with_capacity(prefix_len + messages.len() + suffix_len);
        if let Some(prefix) = &self.prefix_messages {
            merged.extend(prefix.iter().cloned());
        }
        merged.extend(messages);
        if let Some(suffix) = &self.suffix_messages {
            merged.extend(suffix.iter().cloned());
        }
        merged
    }

    /// Computes the deterministic content-addressed ID.
    pub fn id(&self) -> String {
        let mut hasher = XxHash3_128::with_seed(0);
        hasher.write(serde_json::to_string(self).unwrap().as_bytes());
        format!("{:0>22}", base62::encode(hasher.finish_128()))
    }
}

/// A validated OpenRouter Agent with its computed content-addressed ID.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.openrouter.Agent")]
pub struct Agent {
    /// The deterministic content-addressed ID (22-character base62 string).
    pub id: String,
    /// The normalized configuration.
    #[serde(flatten)]
    pub base: AgentBase,
}

impl TryFrom<AgentBase> for Agent {
    type Error = String;
    fn try_from(mut base: AgentBase) -> Result<Self, Self::Error> {
        base.prepare();
        base.validate()?;
        let id = base.id();
        Ok(Agent { id, base })
    }
}
