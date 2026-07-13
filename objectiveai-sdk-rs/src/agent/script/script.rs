//! The script code definition — a `type`-tagged enum flattened into
//! [`AgentBase`](super::AgentBase).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The code a script agent executes on the CLIENT, discriminated by a
/// required `type` field (no default). Flattened into the agent base,
/// so the wire shape is `{"upstream":"script","type":"python","python":"…",…}`.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "agent.script.Script")]
pub enum Script {
    /// Python code executed on the client's embedded runtime — the
    /// SAME shared runtime the `python` command uses. The code
    /// receives the FULL conversation (a messages array, continuation
    /// included) as the `input` global and must output an array of
    /// [`OutputMessage`](super::OutputMessage)s (assistant/tool only).
    #[schemars(title = "Python")]
    Python {
        /// The python source. Preserved verbatim — never normalized
        /// (whitespace is significant).
        python: String,
    },
}

impl Script {
    /// Validates the script definition.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Script::Python { python } => {
                if python.is_empty() {
                    return Err("`python` must not be empty".to_string());
                }
            }
        }
        Ok(())
    }
}
