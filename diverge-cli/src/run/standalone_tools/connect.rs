//! `diverge run standalone-tools connect`: serving a tool the daemon
//! runs to another daemon: the scope that daemon's agents reach the
//! tool through, over the daemon connection, as
//! [`connect`](diverge_sdk::daemon::endpoints::standalone_tools::connect)
//! states it.

use clap::{Args, ValueEnum};
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::request;
use diverge_sdk::wire::client::handle::Handle;

use crate::run::{Emitter, Error, body, shape};

/// Serving a tool the daemon runs to another daemon: the scope that
/// daemon's agents reach the tool through, over the daemon connection.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The MCP exchange to put through the connected tool: the run
    /// connects, puts this one exchange, prints its result, and
    /// disconnects.
    #[arg(long, value_name = "METHOD")]
    pub mcp: Mcp,
    /// The exchange's MCP request, as JSON: the method's params as MCP
    /// names them, decoded as such — absent for a list, required for a
    /// call or a read.
    #[arg(long = "mcp-request", value_name = "JSON")]
    pub mcp_request: Option<String>,
}

/// The tool connected to, the one exchange put and its result
/// printed, and the scope disconnected.
pub async fn run(
    arguments: Arguments,
    handle: &Handle,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let request: request::Frame = body(arguments.body.as_deref())?;
    shape::mcp(
        handle,
        &request,
        arguments.mcp,
        arguments.mcp_request.as_deref(),
        emitter,
    )
    .await
}

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
