//! Web search.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Whether, and how, the model may search the web: Codex's
/// `web_search`, its four values as Codex spells them. Codex performs
/// the search itself — it is a capability of the upstream, not a tool
/// the caller serves — so a search never appears on the stream as
/// one of the caller's tool calls. The harness writes `disabled` when
/// the agent says nothing.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum WebSearch {
    /// No search at all.
    Disabled,
    /// Search over a cached index.
    Cached,
    /// Search over the live index.
    Indexed,
    /// Live fetches of the pages themselves.
    Live,
}
