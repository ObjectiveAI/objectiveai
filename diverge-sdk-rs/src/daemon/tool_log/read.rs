//! A resource read, as the tool's log keeps it.

use serde::{Deserialize, Serialize};

use crate::shared::mcp::attestation;

/// A `resources/read` the daemon routed into the tool: who asked and
/// which resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Read {
    /// Who asked: an agent's container, or a client over a connect.
    pub caller: attestation::Caller,
    /// The resource's URI.
    pub uri: String,
}
