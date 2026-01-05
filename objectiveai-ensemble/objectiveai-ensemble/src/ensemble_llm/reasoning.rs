use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Reasoning {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<ReasoningEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_verbosity: Option<ReasoningSummaryVerbosity>,
}

impl Reasoning {
    pub fn prepare(mut self) -> Option<Self> {
        if let Some(0) = self.max_tokens {
            self.max_tokens = None;
        }
        if let Some(ReasoningSummaryVerbosity::Auto) = self.summary_verbosity {
            self.summary_verbosity = None;
        }
        if self.enabled.is_none()
            && (self.max_tokens.is_some()
                || self.effort.is_some()
                || self.summary_verbosity.is_some())
        {
            self.enabled = Some(true);
        }
        if self.enabled.is_some()
            || self.max_tokens.is_some()
            || self.effort.is_some()
            || self.summary_verbosity.is_some()
        {
            Some(self)
        } else {
            None
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self
            .max_tokens
            .is_some_and(|max_tokens| max_tokens > i32::MAX as u64)
        {
            Err(format!(
                "`reasoning.max_tokens` must be at most {}",
                i32::MAX
            ))
        } else if let Some(false) = self.enabled {
            if self.max_tokens.is_some() {
                Err(
                    "`reasoning.enabled` cannot be false if `reasoning.max_tokens` is set"
                        .to_string(),
                )
            } else if self.effort.is_some() {
                Err("`reasoning.enabled` cannot be false if `reasoning.effort` is set".to_string())
            } else if self.summary_verbosity.is_some() {
                Err(
                    "`reasoning.enabled` cannot be false if `reasoning.summary_verbosity` is set"
                        .to_string(),
                )
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningSummaryVerbosity {
    Auto,
    Concise,
    Detailed,
}
