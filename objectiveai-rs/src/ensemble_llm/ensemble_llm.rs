//! Core Ensemble LLM types and validation logic.

use crate::chat;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use twox_hash::XxHash3_128;

/// The base configuration for an Ensemble LLM (without computed ID).
///
/// This struct contains all configurable parameters for a single LLM within
/// an ensemble. Use [`TryFrom`] to convert to [`EnsembleLlm`] which includes
/// the computed content-addressed ID.
///
/// # Normalization
///
/// Before ID computation, configurations are normalized via [`prepare`](Self::prepare):
/// - Default values are removed (e.g., `temperature: 1.0` → `None`)
/// - Empty collections are removed
/// - Collections are sorted for deterministic ordering
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnsembleLlmBase {
    /// The upstream language model identifier (e.g., `"gpt-4"`, `"claude-3-opus"`).
    pub model: String,

    /// The output mode for vector completions. Ignored for chat completions.
    #[serde(default)]
    pub output_mode: super::OutputMode,

    /// Enable synthetic reasoning for non-reasoning LLMs.
    ///
    /// **Vector completions only.** Ignored for chat completions.
    ///
    /// When enabled, forces the LLM to output a `_think` field before voting,
    /// simulating chain-of-thought reasoning. Requires `output_mode` to be
    /// `JsonSchema` or `ToolCall` (not `Instruction`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthetic_reasoning: Option<bool>,

    /// Number of top log probabilities to return (2-20).
    ///
    /// **Vector completions only.** Ignored for chat completions.
    ///
    /// When set, vector completion votes use token probabilities instead of
    /// discrete selections (if the upstream model provides logprobs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u64>,

    /// Messages prepended to the user's prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix_messages: Option<Vec<chat::completions::request::Message>>,

    /// Messages appended after the user's prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix_messages: Option<Vec<chat::completions::request::Message>>,

    // --- OpenAI-compatible parameters ---
    /// Penalizes tokens based on their frequency in the output so far (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    /// Token ID to bias mapping (-100 to 100). Positive values increase likelihood.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<IndexMap<String, i64>>,
    /// Maximum tokens in the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u64>,
    /// Penalizes tokens based on their presence in the output so far (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    /// Stop sequences that halt generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<super::Stop>,
    /// Sampling temperature (0.0 to 2.0). Higher = more random.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Nucleus sampling probability (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    // --- OpenRouter-specific parameters ---
    /// Maximum tokens (OpenRouter variant of max_completion_tokens).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// Minimum probability threshold for sampling (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f64>,
    /// Provider routing preferences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<super::Provider>,
    /// Reasoning/thinking configuration for supported models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<super::Reasoning>,
    /// Repetition penalty (0.0 to 2.0). Values > 1.0 penalize repetition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f64>,
    /// Top-a sampling parameter (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_a: Option<f64>,
    /// Top-k sampling: only consider the k most likely tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u64>,
    /// Output verbosity hint for supported models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<super::Verbosity>,
}

impl Default for EnsembleLlmBase {
    fn default() -> Self {
        Self {
            model: String::new(),
            output_mode: super::OutputMode::default(),
            synthetic_reasoning: None,
            top_logprobs: None,
            prefix_messages: None,
            suffix_messages: None,
            frequency_penalty: None,
            logit_bias: None,
            max_completion_tokens: None,
            presence_penalty: None,
            stop: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            min_p: None,
            provider: None,
            reasoning: None,
            repetition_penalty: None,
            top_a: None,
            top_k: None,
            verbosity: None,
        }
    }
}

impl EnsembleLlmBase {
    /// Normalizes the configuration for deterministic ID computation.
    ///
    /// This method removes default values, empty collections, and sorts
    /// collections to ensure identical configurations produce identical IDs.
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
                prefix_messages
                    .iter_mut()
                    .for_each(chat::completions::request::Message::prepare);
                Some(prefix_messages)
            }
            None => None,
        };
        self.suffix_messages = match self.suffix_messages.take() {
            Some(suffix_messages) if suffix_messages.is_empty() => None,
            Some(mut suffix_messages) => {
                suffix_messages
                    .iter_mut()
                    .for_each(chat::completions::request::Message::prepare);
                Some(suffix_messages)
            }
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
    }

    /// Validates the configuration.
    ///
    /// Checks that all fields are within valid ranges and that incompatible
    /// options are not combined (e.g., `synthetic_reasoning` with `Instruction` mode).
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
        Ok(())
    }

    /// Computes the deterministic content-addressed ID.
    ///
    /// The ID is a base62-encoded XXHash3-128 hash of the JSON serialization,
    /// padded to 22 characters.
    pub fn id(&self) -> String {
        let mut hasher = XxHash3_128::with_seed(0);
        hasher.write(serde_json::to_string(self).unwrap().as_bytes());
        format!("{:0>22}", base62::encode(hasher.finish_128()))
    }
}

/// A validated Ensemble LLM with its computed content-addressed ID.
///
/// Created by converting from [`EnsembleLlmBase`] via [`TryFrom`].
/// The conversion normalizes and validates the configuration, then computes the ID.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnsembleLlm {
    /// The deterministic content-addressed ID (22-character base62 string).
    pub id: String,
    /// The normalized configuration.
    #[serde(flatten)]
    pub base: EnsembleLlmBase,
}

impl TryFrom<EnsembleLlmBase> for EnsembleLlm {
    type Error = String;
    fn try_from(mut base: EnsembleLlmBase) -> Result<Self, Self::Error> {
        base.prepare();
        base.validate()?;
        let id = base.id();
        Ok(EnsembleLlm { id, base })
    }
}

/// Wrapper that adds fallback LLMs and a count to any LLM type.
///
/// Used to specify how many instances of an LLM to include in an ensemble,
/// along with fallback models to try if the primary fails.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WithFallbacksAndCount<T> {
    /// Number of instances of this LLM in the ensemble. Defaults to 1.
    #[serde(default = "WithFallbacksAndCount::<T>::default_count")]
    pub count: u64,
    /// The primary LLM configuration.
    #[serde(flatten)]
    pub inner: T,
    /// Fallback LLMs to try if the primary fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallbacks: Option<Vec<T>>,
}

impl<T> WithFallbacksAndCount<T> {
    fn default_count() -> u64 {
        1
    }
}

/// An [`EnsembleLlmBase`] with optional fallbacks and count (pre-validation).
pub type EnsembleLlmBaseWithFallbacksAndCount =
    WithFallbacksAndCount<EnsembleLlmBase>;

/// A validated [`EnsembleLlm`] with optional fallbacks and count.
pub type EnsembleLlmWithFallbacksAndCount = WithFallbacksAndCount<EnsembleLlm>;

impl EnsembleLlmWithFallbacksAndCount {
    /// Returns the concatenated IDs of the primary LLM and all fallbacks.
    ///
    /// Used by [`Ensemble`](crate::ensemble::Ensemble) to compute its own
    /// content-addressed ID.
    pub fn full_id(&self) -> String {
        match &self.fallbacks {
            Some(fallbacks) => {
                let mut full_id = String::with_capacity(
                    self.inner.id.len() + fallbacks.len() * 22,
                );
                full_id.push_str(&self.inner.id);
                for fallback in fallbacks {
                    full_id.push_str(&fallback.id);
                }
                full_id
            }
            None => self.inner.id.clone(),
        }
    }

    /// Returns an iterator over the IDs of the primary LLM and all fallbacks.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.inner.id.as_str()).chain(
            self.fallbacks.as_ref().into_iter().flat_map(|fallbacks| {
                fallbacks.iter().map(|fallback| fallback.id.as_str())
            }),
        )
    }
}

impl TryFrom<EnsembleLlmBaseWithFallbacksAndCount>
    for EnsembleLlmWithFallbacksAndCount
{
    type Error = String;
    fn try_from(
        EnsembleLlmBaseWithFallbacksAndCount {
            count,
            inner: base_inner,
            fallbacks: base_fallbacks,
        }: EnsembleLlmBaseWithFallbacksAndCount,
    ) -> Result<Self, Self::Error> {
        let inner = base_inner.try_into()?;
        let fallbacks = match base_fallbacks {
            Some(base_fallbacks) if base_fallbacks.len() > 0 => {
                let mut fallbacks = Vec::with_capacity(base_fallbacks.len());
                for base_fallback in base_fallbacks {
                    fallbacks.push(base_fallback.try_into()?);
                }
                Some(fallbacks)
            }
            _ => None,
        };
        Ok(EnsembleLlmWithFallbacksAndCount {
            count,
            inner,
            fallbacks,
        })
    }
}
