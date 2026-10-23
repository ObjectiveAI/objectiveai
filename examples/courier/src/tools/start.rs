//! `start`: open a conversation with another agent.

use diverge_sdk::daemon::reference;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::send::{Into, send};
use crate::courier::Call;
use crate::failure::Failure;

/// The tool's name.
pub const NAME: &str = "start";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Open a new conversation with another agent, on a topic, with its first message. \
The topic is fixed once the conversation is open. The other agent is told only that it has a new message.";

/// What `start` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// The agent to write to: `{"name": "..."}`, or
    /// `{"template": "...", "index": n}`.
    pub to: reference::Agent,
    /// What the conversation is about.
    pub topic: String,
    /// The first message.
    pub text: String,
}

/// Open the conversation: the agent looked up, never oneself, then
/// the send.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let to = call.state.daemon.agent(&call.state.client, &parameters.to).await?;
    if to.is(&call.caller.template, call.caller.index) {
        return Err(Failure::ToSelf);
    }
    let sent = send(&call, &to, Into::New { topic: parameters.topic.clone() }, &parameters.text).await?;
    Ok(json!({ "conversation": sent.conversation, "message": sent.message, "with": to, "topic": parameters.topic }))
}
