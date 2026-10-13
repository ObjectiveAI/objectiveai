//! What `diverge` does.

use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::{Config, Mcp};

/// The commands. Every request of the daemon's wire goes under
/// [`Run`](Self::Run); the documents in the root are
/// [`Config`](Self::Config)'s.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Put one request to a daemon, as an account of it.
    Run(Run),
    /// Read, rewrite or edit one of the three documents in the root.
    Config(Config),
}

/// One request to a daemon: which daemon, as whom, which request,
/// what it carries, and how its answers are printed. Every answer
/// frame is one compact JSON line, through the jq filter when one is
/// given; nothing stays open past its answer — a list ends at its
/// `listed`, a filetree at its first snapshot, a logs request where
/// it asked to, and a connect once its one MCP exchange is answered.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Run {
    /// The daemon to dial, by its name in the document. Absent, the
    /// document's default.
    #[arg(long, value_name = "NAME")]
    pub daemon: Option<String>,
    /// The account to dial as, by its name under the daemon. Absent,
    /// the daemon's default.
    #[arg(long, value_name = "NAME")]
    pub account: Option<String>,
    /// The request: its name on the daemon's wire, the family and the
    /// endpoint joined by `/` — `agents/create`,
    /// `standalone_tools/templates/list`, `providers/daemons/add`.
    #[arg(value_name = "PATH")]
    pub path: String,
    /// What the request carries, as a JSON object. Absent, `{}`: a
    /// request that carries nothing, or one whose every member has a
    /// default.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// A jq filter run over every answer frame: each value it yields is
    /// a line, and a frame it yields nothing for, or only `null`, is
    /// not printed.
    #[arg(long, value_name = "FILTER")]
    pub jq: Option<String>,
    /// The local file or directory an upload reads from, or a download
    /// writes into. Required for those requests, refused for every
    /// other.
    #[arg(long = "path", value_name = "LOCAL")]
    pub local: Option<PathBuf>,
    /// The MCP exchange to put through a connected tool: required for
    /// `standalone_tools/connect`, which connects, puts this one
    /// exchange, prints its result, and disconnects; refused for every
    /// other request.
    #[arg(long, value_name = "METHOD")]
    pub mcp: Option<Mcp>,
    /// The exchange's MCP request, as JSON: the method's params as MCP
    /// names them, decoded as such — absent for a list, required for a
    /// call or a read.
    #[arg(long = "mcp-request", value_name = "JSON", requires = "mcp")]
    pub mcp_request: Option<String>,
    /// How many seconds the request may take, dialling included; a
    /// request not complete by then is abandoned and the run ends with
    /// an error. Absent, no limit.
    #[arg(long, value_name = "SECONDS")]
    pub timeout: Option<u64>,
}
