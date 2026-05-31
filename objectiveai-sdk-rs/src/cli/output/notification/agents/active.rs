//! `ActiveAgent` — one entry returned by `agents list-active`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One direct-child agent of the parent `agents list-active` was
/// called with, plus the unix-seconds timestamp of its most recent
/// `assistant_response` row in the `messages` table.
///
/// Direct children only — deeper descendants don't appear in the
/// list. See [`crate::filesystem::Client::list_active`].
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.agents.ActiveAgent")]
pub struct ActiveAgent {
    /// Sub-portion of the composite agent id — the trailing
    /// segment(s) past the parent the caller asked about, with
    /// the `{parent}/` prefix stripped. Paste this verbatim into
    /// commands that re-prepend the parent (e.g. `agents
    /// read pending`).
    pub agent_id: String,
    /// Unix-seconds timestamp of this agent's most recent
    /// `assistant_response` row.
    pub last_log: u64,
}
