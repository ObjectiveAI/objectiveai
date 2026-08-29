//! The `rate_limit_event` records: subscription limits moving.

use serde::Deserialize;

/// A `type: "rate_limit_event"` record, emitted when rate-limit
/// information changes for a subscription-authenticated run.
#[derive(Debug, Clone, PartialEq, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RateLimitInfo {
    /// Whether requests pass.
    pub status: RateLimitStatus,
    /// When the limit resets, epoch seconds.
    #[serde(
        rename = "resetsAt"
    )]
    pub resets_at: Option<f64>,
    /// Which limit this is.
    #[serde(
        rename = "rateLimitType"
    )]
    pub rate_limit_type: Option<RateLimitType>,
    /// How much of the limit is used, as a fraction.
    pub utilization: Option<f64>,
    /// Whether OVERAGE requests pass, when overage is in play.
    #[serde(
        rename = "overageStatus"
    )]
    pub overage_status: Option<RateLimitStatus>,
    /// When the overage window resets, epoch seconds.
    #[serde(
        rename = "overageResetsAt"
    )]
    pub overage_resets_at: Option<f64>,
    /// Why overage is unavailable, when it is.
    #[serde(
        rename = "overageDisabledReason"
    )]
    pub overage_disabled_reason: Option<OverageDisabledReason>,
    /// Whether the run is currently billing overage.
    #[serde(
        rename = "isUsingOverage"
    )]
    pub is_using_overage: Option<bool>,
    /// The warning threshold that was crossed, when one was.
    #[serde(
        rename = "surpassedThreshold"
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
/// serde reads through [`String`], which is what lets unit variants
/// and a catch-all share one flat enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(from = "String")]
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


/// Which limit a [`RateLimitInfo`] describes.
///
/// Open-tailed for the reason [`RateLimitStatus`] gives — with one
/// concrete suspect already on file: the source reads this from a
/// "representative claim" header whose vocabulary elsewhere in the
/// same file uses abbreviations (`5h`, `7d`) that never pass through
/// the long-form mapping.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(from = "String")]
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


/// Why overage is unavailable — the schema's thirteen reasons, and
/// the open tail [`RateLimitStatus`] explains. The schema's own
/// `unknown` is kept as a NAMED variant: the server saying "unknown"
/// and the server saying something this crate does not know are
/// different facts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(from = "String")]
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


/// The `rate_limit_event` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitEventType {
    /// The only value.
    #[default]
    RateLimitEvent,
}

impl RateLimitEvent {
    /// Whether this event says requests are being REFUSED — the
    /// info's verdict.
    pub fn rejected(&self) -> bool {
        self.rate_limit_info.rejected()
    }
}

impl RateLimitInfo {
    /// Whether requests are being refused. Only
    /// [`Rejected`](RateLimitStatus::Rejected) is an error: allowed,
    /// warnings and the queueing pair are the limiter coping.
    pub fn rejected(&self) -> bool {
        matches!(self.status, RateLimitStatus::Rejected)
    }

    /// The info as a notification's message body.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "rate_limit",
            "status": self.status.as_str(),
            "rateLimitType": self
                .rate_limit_type
                .as_ref()
                .map(RateLimitType::as_str),
            "resetsAt": self.resets_at,
            "utilization": self.utilization,
        })
    }
}

impl RateLimitStatus {
    /// The wire literal, the open tail verbatim.
    pub fn as_str(&self) -> &str {
        match self {
            RateLimitStatus::Allowed => "allowed",
            RateLimitStatus::AllowedWarning => "allowed_warning",
            RateLimitStatus::Queueing => "queueing",
            RateLimitStatus::QueueingSoft => "queueing_soft",
            RateLimitStatus::Rejected => "rejected",
            RateLimitStatus::Other(value) => value,
        }
    }
}

impl RateLimitType {
    /// The wire literal, the open tail verbatim.
    pub fn as_str(&self) -> &str {
        match self {
            RateLimitType::FiveHour => "five_hour",
            RateLimitType::SevenDay => "seven_day",
            RateLimitType::SevenDayOpus => "seven_day_opus",
            RateLimitType::SevenDaySonnet => "seven_day_sonnet",
            RateLimitType::Overage => "overage",
            RateLimitType::Other(value) => value,
        }
    }
}
