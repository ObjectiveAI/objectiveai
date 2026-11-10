//! Who invoked a tool: the model directly, or a server-side tool.

use serde::Deserialize;

/// `BetaCaller`: the provenance of a tool call, discriminated by its
/// `type` literal — `direct` for the model's own calls, or a
/// versioned `code_execution_*` literal naming the server-side tool
/// that made the call from inside its sandbox.
///
/// The versioned literals are the API's and will grow — a caller
/// this crate does not know lands in [`Other`](Self::Other) with the
/// object preserved verbatim rather than killing the block.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Caller {
    /// The model called the tool itself.
    Direct {
        /// Always `direct`.
        r#type: DirectType,
    },
    /// A code-execution tool called it, 2025-08-25 vintage.
    CodeExecution20250825 {
        /// Always `code_execution_20250825`.
        r#type: CodeExecution20250825Type,
        /// The calling tool's id.
        tool_id: String,
    },
    /// A code-execution tool called it, 2026-01-20 vintage.
    CodeExecution20260120 {
        /// Always `code_execution_20260120`.
        r#type: CodeExecution20260120Type,
        /// The calling tool's id.
        tool_id: String,
    },
    /// A caller newer than this crate, preserved verbatim.
    Other(serde_json::Value),
}

/// The `direct` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DirectType {
    /// The only value.
    #[default]
    Direct,
}

/// The `code_execution_20250825` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum CodeExecution20250825Type {
    /// The only value.
    #[default]
    #[serde(rename = "code_execution_20250825")]
    CodeExecution20250825,
}

/// The `code_execution_20260120` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum CodeExecution20260120Type {
    /// The only value.
    #[default]
    #[serde(rename = "code_execution_20260120")]
    CodeExecution20260120,
}
