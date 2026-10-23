//! `messages`: read one conversation.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::courier::Call;
use crate::failure::Failure;
use crate::store;

/// The tool's name.
pub const NAME: &str = "messages";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Read one of your conversations, oldest first. Only messages you have not read, unless include_read is true. \
Every message shown is then read.";

/// What `messages` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// The conversation, by the id `start` or `conversations` gave.
    pub conversation: i64,
    /// Show the whole conversation, read messages too. Absent, false.
    #[serde(default)]
    pub include_read: bool,
}

/// The conversation's messages: the caller must be in it.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    let with = store::conversations::other(pool, call.template, parameters.conversation, &call.caller).await?;
    let messages = store::messages::read(pool, parameters.conversation, &call.caller, parameters.include_read).await?;
    Ok(json!({ "conversation": parameters.conversation, "with": with, "messages": messages }))
}
