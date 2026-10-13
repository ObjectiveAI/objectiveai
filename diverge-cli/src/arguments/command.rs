//! What `diverge` does.

use clap::{Args, Subcommand};

/// The commands. Every request of the daemon's wire goes under
/// [`Run`](Self::Run).
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Put one request to a daemon, as an account of it.
    Run(Run),
}

/// One request to a daemon: which daemon, as whom, which request, and
/// what it carries. Not yet served: the daemon and the account are
/// selected and the program ends.
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
    /// The request: the endpoint's path on the daemon's wire, as the
    /// family and the endpoint are named there.
    #[arg(value_name = "PATH")]
    pub path: String,
    /// What the request carries, as JSON. Absent, a request that
    /// carries nothing.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
}
