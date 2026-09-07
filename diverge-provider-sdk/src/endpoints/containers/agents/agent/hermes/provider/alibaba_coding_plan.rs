//! Alibaba Cloud's dedicated coding tier.

use serde::{Deserialize, Serialize};

/// Alibaba Cloud's dedicated coding tier.
///
/// APPLICATION: the harness sets `ALIBABA_CODING_PLAN_API_KEY` to
/// [`api_key`](Self::api_key) in the gateway's process environment
/// before Hermes starts.
/// (`DASHSCOPE_API_KEY` is an accepted fallback in Hermes; the
/// specific var is the one set.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    /// The discriminator. Always `alibaba-coding-plan`.
    pub provider: AlibabaCodingPlan,
    /// The API key, applied as `ALIBABA_CODING_PLAN_API_KEY`.
    pub api_key: String,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum AlibabaCodingPlan {
    #[default]
    AlibabaCodingPlan,
}
