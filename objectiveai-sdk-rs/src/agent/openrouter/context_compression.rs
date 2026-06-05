//! Context compression engine for OpenRouter long-context requests.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Which context-compression engine to enable on outgoing OpenRouter
/// chat-completions requests. Maps onto OpenRouter's request-body
/// `plugins` array — see <https://openrouter.ai/docs/features/middle-out>.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.openrouter.ContextCompression")]
pub enum ContextCompression {
    /// Middle-out compression — drops content from the middle of the
    /// conversation when the request would otherwise exceed the
    /// model's context window. The only engine documented today.
    ///
    /// Intentionally no `#[schemars(title)]` here: the builder's
    /// single-variant-anyOf flatten merges the variant's title onto
    /// the parent schema, clobbering the enum's `agent.openrouter
    /// .ContextCompression` rename. Add a per-variant title only
    /// once a second variant lands.
    #[serde(rename = "middle-out")]
    MiddleOut,
}

impl ContextCompression {
    /// Hook used during agent ID computation. There's no default-to-
    /// collapse for this field — every variant is meaningful — so we
    /// pass the value through unchanged.
    pub fn prepare(self) -> Option<Self> {
        Some(self)
    }

    /// Validates the setting (always succeeds — every variant is
    /// inhabited by a documented OpenRouter engine).
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}
