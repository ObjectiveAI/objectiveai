//! `send_message`: the calling agent answers the person.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::chat::Call;
use crate::failure::Failure;
use crate::store;

/// The tool's name.
pub const NAME: &str = "send_message";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Send the person a message. It appears in the person's page at once. Answers the message's id.";

/// What `send_message` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// What to say.
    pub text: String,
}

/// Keep the message, then show it on every page open on the caller's
/// conversation.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    if parameters.text.trim().is_empty() {
        return Err(Failure::Parameters("the message is empty".to_string()));
    }
    let pool = call.state.store.pool(&call.state.client).await?;
    let message = store::agent(pool, &call.caller, &parameters.text).await?;
    call.state.told(&message).await;
    Ok(json!({ "message": message.id }))
}
