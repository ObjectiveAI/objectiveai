use serde::{Deserialize, Serialize};

/// Result of a buffered `Thread.run()` — every item produced during the turn,
/// the assistant's final textual response (or empty string if none), and the
/// usage tally on completion. Mirrors `Turn` in `thread.py:29-36`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Turn {
    pub items: Vec<super::super::ThreadItem>,
    pub final_response: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<super::super::Usage>,
}
