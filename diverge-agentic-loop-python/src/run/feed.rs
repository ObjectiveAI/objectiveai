//! What the script is given.

use rmcp::model::{Resource, Tool};
use serde::Serialize;

use crate::continuation::ContinuationItem;

/// The harness's stdin: one JSON object, the three globals the script
/// reads.
///
/// `input` is the history in its stored form — chunk objects and bare
/// prompt strings, the turn's prompt the last item — so the script
/// sees the conversation exactly as the database keeps it, and a
/// script that inspects the last item finds the latest prompt or the
/// latest tool response. `tools` and `resources` are the MCP listings
/// taken just before this run, as rmcp serializes them.
#[derive(Debug, Clone, Serialize)]
pub struct Feed<'a> {
    /// The whole conversation.
    pub input: &'a [ContinuationItem],
    /// The tools the caller's servers offer, this turn.
    pub tools: &'a [Tool],
    /// The resources the caller's servers offer, this turn.
    pub resources: &'a [Resource],
}
