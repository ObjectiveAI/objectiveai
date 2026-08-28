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
///
/// # Knowns and an open tail, because the vocabulary is the server's
///
/// All three enums here are unchecked casts of raw HTTP response
/// headers in the source — the values come from Anthropic's servers,
/// unvalidated, and the source's own display code is defensive about
/// values it does not know. So each names the known set — the zod's,
/// plus the `queueing` pair the api crate's port attests — and
/// catches anything newer in `Other`, the string preserved verbatim,
/// the same way [`StopReason`](super::message::StopReason) does:
/// serde goes through [`String`] both ways, which is what lets unit
/// variants and a catch-all share one flat enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum RateLimitStatus {
    /// Under the limit.
    Allowed,
    /// Under the limit, but close.
    AllowedWarning,
    /// Requests are being queued.
    Queueing,
    /// Requests are being queued, softly.
    QueueingSoft,
    /// Over it.
    Rejected,
    /// A status newer than this crate, preserved verbatim.
    Other(String),
}

impl From<String> for RateLimitStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "allowed" => RateLimitStatus::Allowed,
            "allowed_warning" => RateLimitStatus::AllowedWarning,
            "queueing" => RateLimitStatus::Queueing,
            "queueing_soft" => RateLimitStatus::QueueingSoft,
            "rejected" => RateLimitStatus::Rejected,
            _ => RateLimitStatus::Other(value),
        }
    }
}

impl From<RateLimitStatus> for String {
    fn from(value: RateLimitStatus) -> Self {
        match value {
            RateLimitStatus::Allowed => "allowed".to_string(),
            RateLimitStatus::AllowedWarning => {
                "allowed_warning".to_string()
            }
            RateLimitStatus::Queueing => "queueing".to_string(),
            RateLimitStatus::QueueingSoft => {
                "queueing_soft".to_string()
            }
            RateLimitStatus::Rejected => "rejected".to_string(),
            RateLimitStatus::Other(value) => value,
        }
    }
}

/// Which limit a [`RateLimitInfo`] describes.
///
/// Open-tailed for the reason [`RateLimitStatus`] gives — with one
/// concrete suspect already on file: the source reads this from a
/// "representative claim" header whose vocabulary elsewhere in the
/// same file uses abbreviations (`5h`, `7d`) that never pass through
/// the long-form mapping.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
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
    /// A limit newer than this crate, preserved verbatim.
    Other(String),
}

impl From<String> for RateLimitType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "five_hour" => RateLimitType::FiveHour,
            "seven_day" => RateLimitType::SevenDay,
            "seven_day_opus" => RateLimitType::SevenDayOpus,
            "seven_day_sonnet" => RateLimitType::SevenDaySonnet,
            "overage" => RateLimitType::Overage,
            _ => RateLimitType::Other(value),
        }
    }
}

impl From<RateLimitType> for String {
    fn from(value: RateLimitType) -> Self {
        match value {
            RateLimitType::FiveHour => "five_hour".to_string(),
            RateLimitType::SevenDay => "seven_day".to_string(),
            RateLimitType::SevenDayOpus => "seven_day_opus".to_string(),
            RateLimitType::SevenDaySonnet => {
                "seven_day_sonnet".to_string()
            }
            RateLimitType::Overage => "overage".to_string(),
            RateLimitType::Other(value) => value,
        }
    }
}

/// Why overage is unavailable — the schema's thirteen reasons, and
/// the open tail [`RateLimitStatus`] explains. The schema's own
/// `unknown` is kept as a NAMED variant: the server saying "unknown"
/// and the server saying something this crate does not know are
/// different facts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
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
    /// The server itself does not know.
    Unknown,
    /// A reason newer than this crate, preserved verbatim.
    Other(String),
}

impl From<String> for OverageDisabledReason {
    fn from(value: String) -> Self {
        match value.as_str() {
            "overage_not_provisioned" => {
                OverageDisabledReason::OverageNotProvisioned
            }
            "org_level_disabled" => {
                OverageDisabledReason::OrgLevelDisabled
            }
            "org_level_disabled_until" => {
                OverageDisabledReason::OrgLevelDisabledUntil
            }
            "out_of_credits" => OverageDisabledReason::OutOfCredits,
            "seat_tier_level_disabled" => {
                OverageDisabledReason::SeatTierLevelDisabled
            }
            "member_level_disabled" => {
                OverageDisabledReason::MemberLevelDisabled
            }
            "seat_tier_zero_credit_limit" => {
                OverageDisabledReason::SeatTierZeroCreditLimit
            }
            "group_zero_credit_limit" => {
                OverageDisabledReason::GroupZeroCreditLimit
            }
            "member_zero_credit_limit" => {
                OverageDisabledReason::MemberZeroCreditLimit
            }
            "org_service_level_disabled" => {
                OverageDisabledReason::OrgServiceLevelDisabled
            }
            "org_service_zero_credit_limit" => {
                OverageDisabledReason::OrgServiceZeroCreditLimit
            }
            "no_limits_configured" => {
                OverageDisabledReason::NoLimitsConfigured
            }
            "unknown" => OverageDisabledReason::Unknown,
            _ => OverageDisabledReason::Other(value),
        }
    }
}

impl From<OverageDisabledReason> for String {
    fn from(value: OverageDisabledReason) -> Self {
        match value {
            OverageDisabledReason::OverageNotProvisioned => {
                "overage_not_provisioned".to_string()
            }
            OverageDisabledReason::OrgLevelDisabled => {
                "org_level_disabled".to_string()
            }
            OverageDisabledReason::OrgLevelDisabledUntil => {
                "org_level_disabled_until".to_string()
            }
            OverageDisabledReason::OutOfCredits => {
                "out_of_credits".to_string()
            }
            OverageDisabledReason::SeatTierLevelDisabled => {
                "seat_tier_level_disabled".to_string()
            }
            OverageDisabledReason::MemberLevelDisabled => {
                "member_level_disabled".to_string()
            }
            OverageDisabledReason::SeatTierZeroCreditLimit => {
                "seat_tier_zero_credit_limit".to_string()
            }
            OverageDisabledReason::GroupZeroCreditLimit => {
                "group_zero_credit_limit".to_string()
            }
            OverageDisabledReason::MemberZeroCreditLimit => {
                "member_zero_credit_limit".to_string()
            }
            OverageDisabledReason::OrgServiceLevelDisabled => {
                "org_service_level_disabled".to_string()
            }
            OverageDisabledReason::OrgServiceZeroCreditLimit => {
                "org_service_zero_credit_limit".to_string()
            }
            OverageDisabledReason::NoLimitsConfigured => {
                "no_limits_configured".to_string()
            }
            OverageDisabledReason::Unknown => "unknown".to_string(),
            OverageDisabledReason::Other(value) => value,
        }
    }
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
