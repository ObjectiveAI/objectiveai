//! `set_alarm`: arm an alarm for the caller.

use chrono::Utc;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::alarm::Call;
use crate::failure::Failure;
use crate::schedule::When;
use crate::store;

/// The tool's name.
pub const NAME: &str = "set_alarm";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Set an alarm that wakes you with a note: give exactly one of `at` (an RFC 3339 time), `in_seconds`, \
or `cron` (a five-field pattern, minute hour day-of-month month day-of-week, in UTC) for one that recurs. When it is due you are \
sent `<alarm>`, the note, `</alarm>`, even if you were not running. Answers the alarm's id and when it is first due.";

/// What `set_alarm` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// What the alarm says when it fires: why you set it.
    pub note: String,
    /// Fire once, at this RFC 3339 time.
    #[serde(default)]
    pub at: Option<String>,
    /// Fire once, this many seconds from now.
    #[serde(default)]
    pub in_seconds: Option<u64>,
    /// Fire on every match of this five-field cron pattern, in UTC.
    #[serde(default)]
    pub cron: Option<String>,
}

/// Arm the alarm, and wake the scheduler to sleep until the right
/// one.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let when = When::of(parameters.at.as_deref(), parameters.in_seconds, parameters.cron.as_deref())?;
    let due = when.first(Utc::now())?;
    let pool = call.state.store.pool(&call.state.client).await?;
    let id = store::alarms::create(pool, &call.caller, &parameters.note, when.cron(), due).await?;
    call.state.wake.notify_one();
    Ok(json!({ "alarm": id, "due": due }))
}
