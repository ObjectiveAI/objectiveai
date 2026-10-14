//! The `run` command: one request to a daemon, its answers printed.
//!
//! `diverge run <family> [<family>…] <request> [BODY]`: the tree of
//! subcommands is the daemon's own tree of families and requests —
//! `agents create`, `standalone-tools templates list`,
//! `providers daemons add` — and the tree of files under this module
//! is the same tree, one folder per family and one file per request,
//! each file holding the request's arguments and its run. What a
//! request alone takes is on its own leaf: the eight that read or
//! write files take `--path`, the connect holds its scope for the
//! one MCP exchange its own subcommand names; nothing is
//! checked at run time that the command line did not already shape.
//!
//! The selected daemon is [`dial`]ed as the selected account; the
//! leaf decodes its [`body`] as the SDK's own request type and puts it
//! through the [`shape`] its exchange has, printing every answer
//! frame through the [`Emitter`] as one compact JSON line, each
//! through the jq filter when there is one. Nothing stays open past
//! its answer: a list ends at its `listed` frame, a filetree at its
//! first snapshot, a logs request where it asked to, and a connect
//! once the one MCP exchange its subcommand names is answered. A
//! timeout abandons the
//! request and ends the run with [`Error::Timeout`].
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

use std::time::Duration;

use clap::{Args, Subcommand};
use diverge_sdk::wire::client::handle::Handle;

use crate::config::Selected;

mod body;
mod dial;
mod emit;
mod error;
mod refusing;

pub mod shape;

pub mod accounts;
pub mod agents;
pub mod dependency_tools;
pub mod postgres;
pub mod providers;
pub mod roles;
pub mod standalone_tools;
pub mod volumes;

pub use body::*;
pub use dial::*;
pub use emit::*;
pub use error::*;
pub use refusing::*;

/// The command: which daemon, as whom, how the answers are printed,
/// how long it may take, and the request.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Run {
    /// The daemon to dial, by its name in the document. Absent, the
    /// document's default.
    #[arg(long, global = true, value_name = "NAME")]
    pub daemon: Option<String>,
    /// The account to dial as, by its name under the daemon. Absent,
    /// the daemon's default.
    #[arg(long, global = true, value_name = "NAME")]
    pub account: Option<String>,
    /// A jq filter run over every answer frame: each value it yields is
    /// a line, and a frame it yields nothing for, or only `null`, is
    /// not printed.
    #[arg(long, global = true, value_name = "FILTER")]
    pub jq: Option<String>,
    /// How many seconds the request may take, dialling included; a
    /// request not complete by then is abandoned and the run ends with
    /// an error. Absent, no limit.
    #[arg(long, global = true, value_name = "SECONDS")]
    pub timeout: Option<u64>,
    /// The request, by its family and its name.
    #[command(subcommand)]
    pub command: Command,
}

/// The families: one folder each.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// The requests over accounts.
    #[command(subcommand)]
    Accounts(accounts::Accounts),
    /// The requests over agents, and their templates.
    #[command(subcommand)]
    Agents(agents::Agents),
    /// The requests over dependency tools.
    #[command(subcommand)]
    DependencyTools(dependency_tools::DependencyTools),
    /// The requests over the database.
    #[command(subcommand)]
    Postgres(postgres::Postgres),
    /// The requests over providers: those dialled, those that dial in,
    /// and daemons reached through them.
    #[command(subcommand)]
    Providers(providers::Providers),
    /// The requests over roles.
    #[command(subcommand)]
    Roles(roles::Roles),
    /// The requests over standalone tools, and their templates.
    #[command(subcommand)]
    StandaloneTools(standalone_tools::StandaloneTools),
    /// The requests over volumes.
    #[command(subcommand)]
    Volumes(volumes::Volumes),
}

/// Put the request `run` names to the daemon `selected` names, as
/// its account: the daemon dialled, the leaf run — within
/// `--timeout`, when one is given — and every answer printed.
pub async fn run(selected: &Selected<'_>, run: Run) -> Result<(), Error> {
    let mut emitter = Emitter::new(run.jq);
    let exchange = async {
        let handle = dial(&selected.daemon.address, &selected.account.key).await?;
        dispatch(run.command, &handle, &mut emitter).await
    };
    match run.timeout {
        None => exchange.await,
        Some(seconds) => match tokio::time::timeout(Duration::from_secs(seconds), exchange).await {
            Ok(outcome) => outcome,
            Err(_) => Err(Error::Timeout(seconds)),
        },
    }
}

/// The request put, by its family.
async fn dispatch(command: Command, handle: &Handle, emitter: &mut Emitter) -> Result<(), Error> {
    match command {
        Command::Accounts(command) => accounts::run(command, handle, emitter).await,
        Command::Agents(command) => agents::run(command, handle, emitter).await,
        Command::DependencyTools(command) => dependency_tools::run(command, handle, emitter).await,
        Command::Postgres(command) => postgres::run(command, handle, emitter).await,
        Command::Providers(command) => providers::run(command, handle, emitter).await,
        Command::Roles(command) => roles::run(command, handle, emitter).await,
        Command::StandaloneTools(command) => standalone_tools::run(command, handle, emitter).await,
        Command::Volumes(command) => volumes::run(command, handle, emitter).await,
    }
}
