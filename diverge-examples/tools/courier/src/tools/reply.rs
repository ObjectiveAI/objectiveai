//! `reply`: answer inside a conversation.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::send::{Into, send};
use crate::courier::Call;
use crate::failure::Failure;
use crate::store;

/// The tool's name.
pub const NAME: &str = "reply";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Add a message to one of your conversations. The other agent is told only that it has a new message.";

/// What `reply` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// The conversation, by the id `create_conversation` or
    /// `list_conversations` gave.
    pub conversation: i64,
    /// The message.
    pub text: String,
}

/// Answer: the caller must be in the conversation; the other agent in
/// it is the one notified.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    let to = store::conversations::other(pool, call.template, parameters.conversation, &call.caller).await?;
    let sent = send(&call, &to, Into::Existing(parameters.conversation), &parameters.text).await?;
    Ok(json!({ "conversation": sent.conversation, "message": sent.message, "with": to }))
}
