//! MCP session-id constants shared across SDK / cli / cli-stream / mcp.
//!
//! Forms in this module are kept in sync by hand:
//!
//! - [`MCP_SESSION_ID_HEADER`] — wire HTTP header name (the MCP
//!   Streamable-HTTP standard spelling, mixed-case dashes).
//! - [`MCP_SESSION_ID_ENV`] — environment variable form, used to
//!   propagate the session id into a cli subprocess (e.g. a tool the
//!   cli spawns) without re-encoding it as a header.
//! - [`OBJECTIVEAI_MCP_ENV`] — boolean marker set by `objectiveai-mcp`
//!   at startup and re-emitted by the cli onto every plugin/tool
//!   subprocess it spawns. Lets descendants know they're running under
//!   an MCP server without sniffing process ancestry.

/// HTTP header name carrying the MCP session id on requests between
/// MCP clients and servers. Matches the casing used by the upstream
/// rmcp transport.
pub const MCP_SESSION_ID_HEADER: &str = "Mcp-Session-Id";

/// Environment variable spelling of the same id. Used to forward the
/// session id from the cli into tool subprocesses it spawns.
pub const MCP_SESSION_ID_ENV: &str = "MCP_SESSION_ID";

/// Boolean marker advertising "this process is running under
/// `objectiveai-mcp`". Set by `objectiveai-mcp`'s `main` at startup,
/// re-emitted by `objectiveai-cli` onto every plugin / tool subprocess
/// it spawns when the value is true.
pub const OBJECTIVEAI_MCP_ENV: &str = "OBJECTIVEAI_MCP";
