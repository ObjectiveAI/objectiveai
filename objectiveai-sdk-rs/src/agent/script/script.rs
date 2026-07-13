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
    // NOTE: no variant-level `#[schemars(title = "...")]`. This is a
    // single-variant enum, which schemars collapses to the lone
    // variant's schema and HOISTS the variant title to the schema's
    // top-level `title` — a title of "Python" makes the JS codegen
    // route this type to `src/python.ts` and leaves every
    // `agent.script.Script` reference dangling. Leaving it off lets
    // the type's `rename` drive the title, matching
    // `ClientLaboratoryType`.
    /// Python code executed on the client's embedded runtime — the
    /// SAME shared runtime the `python` command uses. The code
    /// receives the FULL conversation (a messages array, continuation
    /// included) as the `input` global and must output an array of
    /// [`OutputMessage`](super::OutputMessage)s (assistant/tool only).
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
