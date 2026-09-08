//! Provider routing options: quantization.

use crate::agent;
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
impl From<agent::ProviderQuantization> for ProviderQuantization {
    fn from(quantization: agent::ProviderQuantization) -> Self {
        match quantization {
            agent::ProviderQuantization::Int4 => {
                ProviderQuantization::Int4
            }
            agent::ProviderQuantization::Int8 => {
                ProviderQuantization::Int8
            }
            agent::ProviderQuantization::Fp4 => ProviderQuantization::Fp4,
            agent::ProviderQuantization::Fp6 => ProviderQuantization::Fp6,
            agent::ProviderQuantization::Fp8 => ProviderQuantization::Fp8,
            agent::ProviderQuantization::Fp16 => {
                ProviderQuantization::Fp16
            }
            agent::ProviderQuantization::Bf16 => {
                ProviderQuantization::Bf16
            }
            agent::ProviderQuantization::Fp32 => {
                ProviderQuantization::Fp32
            }
            agent::ProviderQuantization::Unknown => {
                ProviderQuantization::Unknown
            }
        }
    }
}
