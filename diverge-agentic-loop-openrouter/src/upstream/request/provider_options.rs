//! Provider routing options: quantization.

use serde::{Deserialize, Serialize};

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
