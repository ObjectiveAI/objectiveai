//! The `rate_limit_event` records: subscription limits moving.

use serde::{Deserialize, Serialize};

/// A `type: "rate_limit_event"` record, emitted when rate-limit
/// information changes for a subscription-authenticated run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateLimitEvent {
    /// Always `rate_limit_event`.
    pub r#type: RateLimitEventType,
    /// The new information.
    pub rate_limit_info: RateLimitInfo,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}

/// Where the subscription's limits stand.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// Whether requests pass.
    pub status: RateLimitStatus,
    /// When the limit resets, epoch seconds.
    #[serde(
        rename = "resetsAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub resets_at: Option<f64>,
    /// Which limit this is.
    #[serde(
        rename = "rateLimitType",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub rate_limit_type: Option<RateLimitType>,
    /// How much of the limit is used, as a fraction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub utilization: Option<f64>,
    /// Whether OVERAGE requests pass, when overage is in play.
    #[serde(
        rename = "overageStatus",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub overage_status: Option<RateLimitStatus>,
    /// When the overage window resets, epoch seconds.
    #[serde(
        rename = "overageResetsAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub overage_resets_at: Option<f64>,
    /// Why overage is unavailable, when it is.
    #[serde(
        rename = "overageDisabledReason",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub overage_disabled_reason: Option<OverageDisabledReason>,
    /// Whether the run is currently billing overage.
    #[serde(
        rename = "isUsingOverage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub is_using_overage: Option<bool>,
    /// The warning threshold that was crossed, when one was.
    #[serde(
        rename = "surpassedThreshold",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub surpassed_threshold: Option<f64>,
}

/// Whether requests pass a limit.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitStatus {
    /// Under the limit.
    Allowed,
    /// Under the limit, but close.
    AllowedWarning,
    /// Over it.
    Rejected,
}

/// Which limit a [`RateLimitInfo`] describes.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitType {
    /// The rolling five-hour window.
    FiveHour,
    /// The rolling seven-day window.
    SevenDay,
    /// The seven-day Opus window.
    SevenDayOpus,
    /// The seven-day Sonnet window.
    SevenDaySonnet,
    /// The overage pool.
    Overage,
}

/// Why overage is unavailable — the source's own thirteen reasons.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum OverageDisabledReason {
    /// Overage was never provisioned.
    OverageNotProvisioned,
    /// The organization turned it off.
    OrgLevelDisabled,
    /// The organization turned it off until a date.
    OrgLevelDisabledUntil,
    /// No credits left.
    OutOfCredits,
    /// The seat tier turned it off.
    SeatTierLevelDisabled,
    /// The member turned it off.
    MemberLevelDisabled,
    /// The seat tier's credit limit is zero.
    SeatTierZeroCreditLimit,
    /// The group's credit limit is zero.
    GroupZeroCreditLimit,
    /// The member's credit limit is zero.
    MemberZeroCreditLimit,
    /// The organization's service level turned it off.
    OrgServiceLevelDisabled,
    /// The organization's service credit limit is zero.
    OrgServiceZeroCreditLimit,
    /// No limits are configured at all.
    NoLimitsConfigured,
    /// Something else.
    Unknown,
}

/// The `rate_limit_event` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitEventType {
    /// The only value.
    #[default]
    RateLimitEvent,
}
