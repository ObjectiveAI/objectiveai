use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SDKRateLimitEvent {
    pub r#type: RateLimitEventType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit_info: Option<RateLimitInfo>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitEventType {
    RateLimitEvent,
    RateLimit,
}

/// Payload accompanying a `rate_limit_event`. The `claude` CLI emits
/// `resetsAt` in camelCase as Unix seconds.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RateLimitInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<RateLimitStatus>,
    #[serde(
        rename = "resetsAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub resets_at: Option<u64>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitStatus {
    Allowed,
    AllowedWarning,
    Queueing,
    QueueingSoft,
    Rejected,
}
