//! The MCP exchange a connect run puts.

use clap::ValueEnum;

/// The four unary MCP exchanges a connected tool answers, as `--mcp`
/// names one: the notifications stream is no exchange of a command
/// line's and is not among them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Mcp {
    /// The tool's MCP tools; the request, absent or `{"cursor":…}`.
    ListTools,
    /// The tool's MCP resources; the request, absent or `{"cursor":…}`.
    ListResources,
    /// One call; the request `{"name":…,"arguments":{…}}`.
    CallTool,
    /// One resource read; the request `{"uri":…}`.
    ReadResource,
}
