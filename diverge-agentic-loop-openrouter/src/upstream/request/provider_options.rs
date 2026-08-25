//! Provider routing options: collection, sort, price, quantization.

use serde::{Deserialize, Serialize};

/// Data collection policy for providers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDataCollection {
    /// Do not allow data collection.
    Deny,
    /// Allow data collection.
    Allow,
}

/// How to sort/prioritize providers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSort {
    /// Prioritize by price (cheapest first).
    Price,
    /// Prioritize by throughput (fastest first).
    Throughput,
    /// Prioritize by latency (lowest first).
    Latency,
}

/// Maximum price constraints per token type.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProviderMaxPrice {
    /// Maximum price per prompt token.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::upstream::serde_util::option_decimal"
    )]
    pub prompt: Option<rust_decimal::Decimal>,
    /// Maximum price per completion token.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::upstream::serde_util::option_decimal"
    )]
    pub completion: Option<rust_decimal::Decimal>,
    /// Maximum price per image.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::upstream::serde_util::option_decimal"
    )]
    pub image: Option<rust_decimal::Decimal>,
    /// Maximum price per audio second.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::upstream::serde_util::option_decimal"
    )]
    pub audio: Option<rust_decimal::Decimal>,
    /// Maximum price per request.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::upstream::serde_util::option_decimal"
    )]
    pub request: Option<rust_decimal::Decimal>,
}

/// Model quantization levels for provider filtering.
///
/// Quantization reduces model precision to decrease memory usage and
/// increase inference speed, potentially at the cost of output quality.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
)]
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
