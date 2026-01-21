//! Provider routing preferences for Ensemble LLMs.
//!
//! These settings control how requests are routed to upstream model providers
//! (e.g., via OpenRouter).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Provider routing preferences.
///
/// Controls which providers are used and in what order when routing
/// requests to upstream model hosts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
pub struct Provider {
    /// Whether to allow fallback to other providers if preferred ones fail.
    /// Defaults to `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    /// Whether to require that the provider supports all request parameters.
    /// Defaults to `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<bool>,
    /// Preferred provider order. Earlier providers are tried first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    /// Exclusive list of allowed providers. If set, only these providers are used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only: Option<Vec<String>>,
    /// Providers to exclude from routing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    /// Allowed model quantization levels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantizations: Option<Vec<ProviderQuantization>>,
}

impl Provider {
    pub fn prepare(mut self) -> Option<Self> {
        if let Some(true) = self.allow_fallbacks {
            self.allow_fallbacks = None;
        }
        if let Some(false) = self.require_parameters {
            self.require_parameters = None;
        }
        if let Some(order) = &mut self.order {
            if order.is_empty() {
                self.order = None;
            } else {
                let mut dedup = HashSet::with_capacity(order.len());
                order.retain(|provider| dedup.insert(provider.clone()));
            }
        }
        if let Some(only) = &mut self.only {
            if only.is_empty() {
                self.only = None;
            } else {
                only.sort();
                only.dedup();
            }
        }
        if let Some(ignore) = &mut self.ignore {
            if ignore.is_empty() {
                self.ignore = None;
            } else {
                ignore.sort();
                ignore.dedup();
            }
        }
        if let Some(quantizations) = &mut self.quantizations {
            if quantizations.is_empty() {
                self.quantizations = None;
            } else {
                quantizations.sort();
                quantizations.dedup();
            }
        }
        if self.allow_fallbacks.is_some()
            || self.require_parameters.is_some()
            || self.order.is_some()
            || self.only.is_some()
            || self.ignore.is_some()
            || self.quantizations.is_some()
        {
            Some(self)
        } else {
            None
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.order.iter().any(|s| s.is_empty()) {
            Err("`provider.order` strings cannot be empty".to_string())
        } else if self.only.iter().any(|s| s.is_empty()) {
            Err("`provider.only` strings cannot be empty".to_string())
        } else if self.ignore.iter().any(|s| s.is_empty()) {
            Err("`provider.ignore` strings cannot be empty".to_string())
        } else {
            Ok(())
        }
    }
}

/// Model quantization levels for provider filtering.
///
/// Quantization reduces model precision to decrease memory usage and
/// increase inference speed, potentially at the cost of output quality.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProviderQuantization {
    /// 4-bit integer quantization.
    Int4,
    /// 8-bit integer quantization.
    Int8,
    /// 4-bit floating point quantization.
    Fp4,
    /// 6-bit floating point quantization.
    Fp6,
    /// 8-bit floating point quantization.
    Fp8,
    /// 16-bit floating point (half precision).
    Fp16,
    /// 16-bit brain floating point.
    Bf16,
    /// 32-bit floating point (full precision).
    Fp32,
    /// Unknown quantization level.
    Unknown,
}

impl PartialOrd for ProviderQuantization {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some((*self as u16).cmp(&(*other as u16)))
    }
}

impl Ord for ProviderQuantization {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
