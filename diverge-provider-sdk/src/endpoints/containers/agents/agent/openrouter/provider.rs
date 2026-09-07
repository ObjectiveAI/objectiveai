//! Provider routing preferences.

use serde::{Deserialize, Serialize};

/// Which backing providers OpenRouter may route to, and in what order.
///
/// OpenRouter fronts many providers for one model, and they differ in
/// price, throughput and quantization — so "which model" does not
/// fully determine what answers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Provider {
    /// Whether to fall back to a provider outside the preferences when
    /// none of them can serve the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    /// Only route to providers supporting every parameter sent.
    /// Without this, unsupported parameters are silently dropped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<bool>,
    /// Preferred providers, most preferred first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    /// Route to these and no others.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub only: Option<Vec<String>>,
    /// Never route to these.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    /// Acceptable weight quantizations. A more aggressively quantized
    /// model is cheaper and measurably different.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantizations: Option<Vec<ProviderQuantization>>,
}

/// A weight quantization level.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
    Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ProviderQuantization {
    /// 4-bit integer.
    Int4,
    /// 8-bit integer.
    Int8,
    /// 4-bit float.
    Fp4,
    /// 6-bit float.
    Fp6,
    /// 8-bit float.
    Fp8,
    /// 16-bit float (half).
    Fp16,
    /// 16-bit brain float.
    Bf16,
    /// 32-bit float (full).
    Fp32,
    /// Not reported.
    #[default]
    Unknown,
}
