//! `conversations`: the caller's inbox.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::courier::Call;
use crate::failure::Failure;
use crate::store;

/// The tool's name.
pub const NAME: &str = "conversations";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "List your conversations, newest message first, each with its id, the other agent, its topic and how many \
messages you have not read. Only conversations with unread messages, unless include_read is true.";

/// What `conversations` takes.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// Also list conversations with nothing unread. Absent, false.
    #[serde(default)]
    pub include_read: bool,
}

/// The inbox.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    let conversations = store::conversations::list(pool, call.template, &call.caller, parameters.include_read).await?;
    Ok(json!({ "conversations": conversations }))
}
