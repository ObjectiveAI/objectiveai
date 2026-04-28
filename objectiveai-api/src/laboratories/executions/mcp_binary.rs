/// Pre-built `objectiveai-mcp-filesystem` binary (linux-musl).
/// Injected into laboratory containers; same architecture as the API server build target.
pub const MCP_FILESYSTEM_BINARY: &[u8] =
    include_bytes!(env!("OBJECTIVEAI_MCP_FILESYSTEM_BINARY_PATH"));
