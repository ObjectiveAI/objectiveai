//! `read_messages`: what the person wrote to the calling agent.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::chat::Call;
use crate::failure::Failure;
use crate::store;

/// The tool's name.
pub const NAME: &str = "read_messages";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Read what the person wrote to you, oldest first: only the messages you have not read yet, or every message when \
`include_read` is true. Every unread message handed back is marked read.";

/// What `read_messages` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// Hand back the messages already read too. Absent, `false`: the
    /// unread ones only.
    #[serde(default)]
    pub include_read: bool,
}

/// The person's messages to the caller, each with when it was written
/// and whether it had been read before this call.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    let messages = store::read(pool, &call.caller, parameters.include_read).await?;
    let messages: Vec<Value> = messages
        .into_iter()
        .map(|message| json!({ "id": message.id, "text": message.body, "from": { "issuer": message.issuer, "subject": message.subject }, "written": message.created, "already_read": message.read }))
        .collect();
    Ok(json!({ "messages": messages }))
}
