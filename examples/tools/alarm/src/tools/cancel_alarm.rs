//! `cancel_alarm`: cancel one of the caller's alarms.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::alarm::Call;
use crate::failure::Failure;
use crate::store;

/// The tool's name.
pub const NAME: &str = "cancel_alarm";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Cancel one of your alarms, by the id `set_alarm` or `list_alarms` gave; a recurring one stops for good.";

/// What `cancel_alarm` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// The alarm, by the id `set_alarm` or `list_alarms` gave.
    pub alarm: i64,
}

/// Cancel the alarm, and wake the scheduler to sleep until the right
/// one.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    store::alarms::cancel(pool, parameters.alarm, &call.caller).await?;
    call.state.wake.notify_one();
    Ok(json!({ "cancelled": parameters.alarm }))
}
