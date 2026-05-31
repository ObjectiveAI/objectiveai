//! `objectiveai-cli-stream` — per-stream subprocess runner for the
//! ObjectiveAI CLI.
//!
//! All configuration is on the command line — no env vars, no
//! on-disk config — so the binary is self-contained for use as a
//! per-call spawn target. Args are grouped into `HttpArgs` (HTTP
//! client construction) and `PipeArgs` (MCP conduit + per-agent
//! named-pipe directory + log root).
//!
//! Today this binary serves the `functions executions create`
//! streaming endpoint. Future endpoints (agent completions, vector
//! completions, function inventions, laboratories executions, etc.)
//! follow the same `<top>/<sub>/<verb>/mod.rs` filesystem layout as
//! `objectiveai-cli`; pipe lifecycle, chunk emission, log writing,
//! and the MCP conduit are factored into shared modules already.

mod agents;
mod api;
mod functions;
mod pipes;
mod streaming;

use clap::{Parser, Subcommand};
use objectiveai_sdk::cli::output::Handle;

use crate::api::{HttpArgs, PipeArgs};

/// Per-stream subprocess runner for the ObjectiveAI CLI.
#[derive(Parser, Debug)]
#[command(
    name = "objectiveai-cli-stream",
    version,
    about = "Per-stream subprocess runner for the ObjectiveAI CLI",
)]
struct Cli {
    #[command(flatten)]
    http: HttpArgs,

    #[command(flatten)]
    pipes: PipeArgs,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
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
    async fn handle(
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

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();
    // Stamp the handle's agent_id from --objectiveai-agent-id so
    // every emitted Notification/Error line carries it — mirrors
    // objectiveai-cli/src/main.rs:16-17.
    let mut handle = Handle::stdout();
    handle.agent_id = cli.http.objectiveai_agent_id.clone();

    cli.command.handle(&cli.http, &cli.pipes, &handle).await
}
