//! A tool call, as the tool's log keeps it.

use serde::{Deserialize, Serialize};

use crate::shared::mcp::attestation;

/// A `tools/call` the daemon routed into the tool: who asked, as the
/// daemon established it, the tool's own name for what was called —
/// never the prefixed name the caller saw — and the arguments, as they
/// came.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Call {
    /// Who asked: an agent's container, or a client over a connect.
    pub caller: attestation::Caller,
    /// The tool's own name for what was called.
    pub name: String,
    /// The arguments, as they came; absent when the call carried none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Map<String, serde_json::Value>>,
}
