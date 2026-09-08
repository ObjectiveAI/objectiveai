//! Provider routing options: quantization.

use diverge_provider_sdk::endpoints::containers::agents::agent::openrouter;
use serde::Serialize;

/// Model quantization levels for provider filtering.
///
/// Quantization reduces model precision to decrease memory usage and
/// increase inference speed, potentially at the cost of output quality.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
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

/// The provider request's quantization, variant for variant.
impl From<openrouter::ProviderQuantization> for ProviderQuantization {
    fn from(quantization: openrouter::ProviderQuantization) -> Self {
        match quantization {
            openrouter::ProviderQuantization::Int4 => {
                ProviderQuantization::Int4
            }
            openrouter::ProviderQuantization::Int8 => {
                ProviderQuantization::Int8
            }
            openrouter::ProviderQuantization::Fp4 => ProviderQuantization::Fp4,
            openrouter::ProviderQuantization::Fp6 => ProviderQuantization::Fp6,
            openrouter::ProviderQuantization::Fp8 => ProviderQuantization::Fp8,
            openrouter::ProviderQuantization::Fp16 => {
                ProviderQuantization::Fp16
            }
            openrouter::ProviderQuantization::Bf16 => {
                ProviderQuantization::Bf16
            }
            openrouter::ProviderQuantization::Fp32 => {
                ProviderQuantization::Fp32
            }
            openrouter::ProviderQuantization::Unknown => {
                ProviderQuantization::Unknown
            }
        }
    }
}
