//! MCP environment markers shared across SDK / cli / mcp.
//!
//! (The former `MCP_SESSION_ID_HEADER`/`MCP_SESSION_ID_ENV` identity
//! propagation was removed — the MCP transport's own `Mcp-Session-Id`
//! handling lives in the rmcp-facing modules, and no session id rides
//! the config/identity plumbing anymore.)

/// Boolean marker advertising "this process is running under
/// `objectiveai-mcp`". Set by `objectiveai-mcp`'s `main` at startup,
/// re-emitted by `objectiveai-cli` onto every plugin / tool subprocess
/// it spawns when the value is true.
pub const OBJECTIVEAI_MCP_ENV: &str = "OBJECTIVEAI_MCP";
