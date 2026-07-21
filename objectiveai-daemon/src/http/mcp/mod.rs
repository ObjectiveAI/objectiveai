//! The daemon's OWN MCP server — the ObjectiveAI CLI exposed as a
//! single root-gated MCP tool, served in-process on the daemon's HTTP
//! router at `/mcp` (rmcp streamable HTTP: POST + GET-SSE). Folded in
//! from the retired standalone `objectiveai-mcp` binary (#276): tool
//! calls execute through [`crate::executor::DaemonCommandExecutor`]
//! instead of spawning a fresh CLI subprocess per command, and there
//! is no separate process, port, or config to manage.

mod agent_args_registry;
pub use agent_args_registry::*;
mod format;
pub use format::*;
#[cfg(test)]
mod format_tests;
mod header_session_manager;
pub use header_session_manager::*;
mod objectiveai;
pub use objectiveai::*;
mod service;
pub use service::*;
