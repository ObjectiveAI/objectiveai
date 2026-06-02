//! Per-agent-instance subprocess runner — the personality the cli takes
//! on when invoked as `objectiveai-cli instance <args…>`. Lives in the
//! same crate as the cli itself so it can use any shared helper, error
//! variant, or wire type directly.
//!
//! All configuration is on the command line — no env vars, no on-disk
//! config — so the runner is self-contained for use as a per-call spawn
//! target. Args are grouped into [`HttpArgs`] (HTTP client construction)
//! and [`PipeArgs`] (MCP conduit + per-agent named-pipe directory + log
//! root). Both are `#[command(flatten)]`d onto the `Instance` variant of
//! the cli's top-level `Commands` enum.
//!
//! Today this serves the streaming endpoints (`agents spawn`,
//! `functions executions create`, `functions inventions recursive
//! create`). Future endpoints follow the same
//! `<top>/<sub>/<verb>/mod.rs` filesystem layout as `objectiveai-cli`;
//! pipe lifecycle, chunk emission, log writing, and the MCP conduit are
//! factored into shared modules already.

mod agents;
pub(crate) mod api;
mod functions;
mod pipes;
mod streaming;

pub use self::api::{HttpArgs, PipeArgs};

use clap::Subcommand;
use objectiveai_sdk::cli::output::Handle;

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Agents management
    Agents {
        #[command(subcommand)]
        command: agents::Commands,
    },
    /// Functions management
    Functions {
        #[command(subcommand)]
        command: functions::Commands,
    },
}

impl Commands {
    pub async fn handle(
        self,
        http: &HttpArgs,
        pipes: &PipeArgs,
        handle: &Handle,
    ) -> Result<(), String> {
        match self {
            Commands::Agents { command } => command.handle(http, pipes, handle).await,
            Commands::Functions { command } => command.handle(http, pipes, handle).await,
        }
    }
}
