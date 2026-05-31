//! [`NotificationValue`] — discriminated enum of every
//! notification payload the cli emits.
//!
//! Lives one level inside [`super::Notification`]'s `value` field,
//! so the wire shape is
//! `{"type":"notification","value":{"kind":"<variant>",<fields>}}`.
//! The `kind` discriminator was added so a downstream consumer can
//! do a single `serde_json::from_str::<Output>(line)` and dispatch
//! on the variant without already knowing which payload to expect.
//!
//! Every concrete struct the cli emits gets a typed variant.
//! Generic and one-off payloads (`Items<T>`, `Value<V>`, raw
//! `serde_json::Value`, api-call passthrough `Resp`/`Chunk`) route
//! through the single [`NotificationValue::Other`] catch-all, which
//! is a `serde_json::Map` that flattens directly alongside `kind`
//! — no inner field wrapper.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    ActiveAgent, Agent, AgentItems, Cleared, Detached, Execution, Function, Help,
    Installed, Instructions, Inventions, JqResults, Laboratory, LogContent,
    LogStreamReady, Mcp, Me, MessageDelivered, MessageQueued, Ok, Pair, Plugin,
    Plugins, Profile, Published, Schema, Schemas, Spawned, State, Swarm, Tool,
    ToolLine, Tools, Updater, ViewerSendResult,
};

/// One emitted notification payload. The `kind` tag discriminates
/// the variant. See module-level docs for the wire shape.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[schemars(rename = "cli.output.notification.NotificationValue")]
pub enum NotificationValue {
    // Agents
    ActiveAgent(ActiveAgent),
    Agent(Agent),
    AgentItems(AgentItems),
    MessageDelivered(MessageDelivered),
    MessageQueued(MessageQueued),
    Spawned(Spawned),

    // API
    Detached(Detached),

    // Functions
    Execution(Execution),
    Function(Function),
    Inventions(Inventions),
    Pair(Pair),
    Profile(Profile),
    State(State),

    // Laboratories
    Laboratory(Laboratory),

    // Swarms
    Swarm(Swarm),

    // Shared / multi-command
    Cleared(Cleared),
    Help(Help),
    Installed(Installed),
    Instructions(Instructions),
    JqResults(JqResults),
    LogContent(LogContent),
    LogStreamReady(LogStreamReady),
    Mcp(Mcp),
    Me(Me),
    Ok(Ok),
    Plugin(Plugin),
    Plugins(Plugins),
    Published(Published),
    Schema(Schema),
    Schemas(Schemas),
    Tool(Tool),
    ToolLine(ToolLine),
    Tools(Tools),
    Updater(Updater),
    ViewerSendResult(ViewerSendResult),

    /// Single catch-all for anything that doesn't get a typed
    /// variant: generic emits (`Items<T>`, `Value<V>`),
    /// api/call.rs passthrough (`Resp`, `Chunk`), and raw
    /// `serde_json::Value`. The map's keys flatten directly
    /// alongside `kind` — there is no inner field wrapping.
    ///
    /// Construct via [`NotificationValue::other`]. The payload
    /// must serialize to a JSON object (so its entries can sit at
    /// the same level as `kind`), and its keys cannot include
    /// `"kind"` (would collide with the discriminator).
    ///
    /// Wire examples:
    ///   `{"kind":"other","items":[…]}`        (Items<T>)
    ///   `{"kind":"other","value":<V>}`        (Value<V>)
    Other(serde_json::Map<String, serde_json::Value>),
}

impl NotificationValue {
    /// Build an `Other` variant from an arbitrary serializable
    /// payload. Panics if the payload doesn't serialize to a JSON
    /// object — `Other` flattens, so non-object payloads have
    /// nowhere to land.
    pub fn other<T: Serialize>(value: &T) -> Self {
        let v = serde_json::to_value(value)
            .expect("NotificationValue::other: payload must serialize");
        match v {
            serde_json::Value::Object(map) => Self::Other(map),
            other => panic!(
                "NotificationValue::other: payload must be a JSON object, got {other:?}"
            ),
        }
    }
}

macro_rules! from_variant {
    ($($v:ident),* $(,)?) => {
        $(
            impl From<$v> for NotificationValue {
                fn from(v: $v) -> Self { Self::$v(v) }
            }
        )*
    };
}

from_variant! {
    ActiveAgent, Agent, AgentItems, MessageDelivered, MessageQueued, Spawned,
    Detached,
    Execution, Function, Inventions, Pair, Profile, State,
    Laboratory,
    Swarm,
    Cleared, Help, Installed, Instructions, JqResults, LogContent, LogStreamReady,
    Mcp, Me, Ok, Plugin, Plugins, Published, Schema, Schemas, Tool, ToolLine, Tools,
    Updater, ViewerSendResult,
}
