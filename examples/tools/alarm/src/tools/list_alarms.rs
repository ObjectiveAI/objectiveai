//! `list_alarms`: the caller's alarms.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::alarm::Call;
use crate::failure::Failure;
use crate::store;

/// The tool's name.
pub const NAME: &str = "list_alarms";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "List your alarms, soonest first, each with its id, note, cron pattern if it recurs, when it is next due, and \
when it was set.";

/// What `list_alarms` takes: nothing.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {}

/// The caller's alarms.
pub async fn call(call: Call<'_>, _parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    let alarms = store::alarms::list(pool, &call.caller).await?;
    Ok(json!({ "alarms": alarms }))
}
