mod message;
mod provider_preferences;
mod reasoning;
mod stop;
mod verbosity;

pub use message::*;
pub use provider_preferences::*;
pub use reasoning::*;
pub use stop::*;
pub use verbosity::*;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use twox_hash::XxHash3_128;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnsembleLlmBase {
    // the upstream language model to use
    pub model: String,

    // output mode
    #[serde(default)]
    pub output_mode: OutputMode,

    // whether to use synthetic reasoning for non-reasoning LLMs
    // excludes output_mode: instruction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthetic_reasoning: Option<bool>,

    // whether to use logprobs
    // changes `vote` to probabilities (if upstream actually provides them)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u64>,

    // messages which will precede the user prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix_messages: Option<Vec<Message>>,

    // messages which will follow the user prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix_messages: Option<Vec<Message>>,

    // openai fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<IndexMap<String, i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    // openrouter fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Reasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_a: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<Verbosity>,
}

impl EnsembleLlmBase {
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
                prefix_messages.iter_mut().for_each(Message::prepare);
                Some(prefix_messages)
            }
            None => None,
        };
        self.suffix_messages = match self.suffix_messages.take() {
            Some(suffix_messages) if suffix_messages.is_empty() => None,
            Some(mut suffix_messages) => {
                suffix_messages.iter_mut().for_each(Message::prepare);
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
            && let OutputMode::Instruction = self.output_mode
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

    pub fn id(&self) -> String {
        let mut hasher = XxHash3_128::with_seed(0);
        hasher.write(serde_json::to_string(self).unwrap().as_bytes());
        format!("{:0>22}", base62::encode(hasher.finish_128()))
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    Instruction, // instructed to output a key
    JsonSchema, // response format json schema containing an enum of possible keys
    ToolCall, // forced tool with argument schema containing an enum of possible keys
}

impl std::default::Default for OutputMode {
    fn default() -> Self {
        OutputMode::Instruction
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnsembleLlm {
    pub id: String,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WithFallbacksAndCount<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    #[serde(flatten)]
    pub inner: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallbacks: Option<Vec<T>>,
}

pub type EnsembleLlmBaseWithFallbacksAndCount =
    WithFallbacksAndCount<EnsembleLlmBase>;

pub type EnsembleLlmWithFallbacksAndCount = WithFallbacksAndCount<EnsembleLlm>;

impl EnsembleLlmWithFallbacksAndCount {
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
