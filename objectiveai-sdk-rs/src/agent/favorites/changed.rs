//! Notification payloads for agent-favorite mutations.
//!
//! Emitted by the cli's `agents favorites config {add,del,edit}` handlers
//! and consumed by the viewer's `/agents/favorites/changed` HTTP route
//! (→ `Event::Inbound` with `sub_type = "agents_favorites_changed"`).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "agent.favorites.ChangedNotification")]
pub struct ChangedNotification {
    pub action: Action,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, JsonSchema)]
#[schemars(rename = "agent.favorites.Action")]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Added,
    Edited,
    Removed,
}
