//! When an alarm is due: a time, a delay, or a cron pattern.
//!
//! The rules:
//! - exactly one of `at`, `in_seconds` and `cron` is given;
//! - `at` is an RFC 3339 time, not in the past;
//! - `in_seconds` is counted from the moment the alarm is set;
//! - `cron` is a five-field pattern (minute, hour, day of month,
//!   month, day of week), read in UTC; the alarm is due at the next
//!   time it matches, strictly after now, and after each firing at the
//!   next time strictly after the firing, so repeats missed while the
//!   tool was down are skipped, never made up.

use chrono::{DateTime, TimeDelta, Utc};
use croner::Cron;
use croner::parser::{CronParser, Seconds, Year};

use crate::failure::Failure;

/// An alarm's time, as it was asked for.
#[derive(Debug, Clone)]
pub enum When {
    /// Once, at this time.
    At(DateTime<Utc>),
    /// Once, this long after it is set.
    In(TimeDelta),
    /// On every match of this pattern, kept as written.
    Cron(String),
}

impl When {
    /// The time asked for, from the three ways of asking: exactly one
    /// given.
    pub fn of(at: Option<&str>, in_seconds: Option<u64>, cron: Option<&str>) -> Result<When, Failure> {
        match (at, in_seconds, cron) {
            (Some(at), None, None) => {
                let at = DateTime::parse_from_rfc3339(at).map_err(|error| Failure::When(format!("`at` is not an RFC 3339 time: {error}")))?;
                Ok(When::At(at.with_timezone(&Utc)))
            }
            (None, Some(seconds), None) => {
                let delay = i64::try_from(seconds).ok().and_then(TimeDelta::try_seconds).ok_or_else(|| Failure::When("`in_seconds` is too large".to_string()))?;
                Ok(When::In(delay))
            }
            (None, None, Some(cron)) => {
                parse(cron)?;
                Ok(When::Cron(cron.to_string()))
            }
            _ => Err(Failure::When("give exactly one of `at`, `in_seconds` and `cron`".to_string())),
        }
    }

    /// When the alarm is first due, set at `now`.
    pub fn first(&self, now: DateTime<Utc>) -> Result<DateTime<Utc>, Failure> {
        match self {
            When::At(at) if *at < now => Err(Failure::When(format!("{at} is in the past"))),
            When::At(at) => Ok(*at),
            When::In(delay) => now.checked_add_signed(*delay).ok_or_else(|| Failure::When("`in_seconds` is too large".to_string())),
            When::Cron(cron) => after(cron, now).ok_or_else(|| Failure::When(format!("`{cron}` never matches again"))),
        }
    }

    /// The pattern, for a recurring alarm.
    pub fn cron(&self) -> Option<&str> {
        match self {
            When::Cron(cron) => Some(cron),
            When::At(_) | When::In(_) => None,
        }
    }
}

/// The next time `cron` matches strictly after `time`, or none if it
/// never does, or no longer parses.
pub fn after(cron: &str, time: DateTime<Utc>) -> Option<DateTime<Utc>> {
    parse(cron).ok()?.find_next_occurrence(&time, false).ok()
}

/// `cron`, parsed as a five-field pattern: a field for seconds or for
/// the year is refused, not read.
fn parse(cron: &str) -> Result<Cron, Failure> {
    CronParser::builder()
        .seconds(Seconds::Disallowed)
        .year(Year::Disallowed)
        .build()
        .parse(cron)
        .map_err(|error| Failure::When(format!("`{cron}` is not a five-field cron pattern: {error}")))
}
