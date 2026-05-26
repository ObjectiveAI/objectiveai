//! `objectiveai-cli-stream` — per-stream subprocess runner for the
//! ObjectiveAI CLI.
//!
//! All configuration is on the command line — no env vars, no
//! on-disk config — so the binary is self-contained for use as a
//! per-call spawn target. Args are grouped into `HttpArgs` (HTTP
//! client construction) and `PipeArgs` (MCP conduit + per-agent
//! named-pipe directory).
//!
//! Today this binary serves the `functions executions` streaming
//! endpoint. Future endpoints (agent completions, vector completions,
//! function inventions, laboratories executions, etc.) plug in as
//! new modules under [`endpoints`]; pipe lifecycle, chunk emission,
//! and the MCP conduit are factored into shared modules already.

mod args;
mod conduit;
mod endpoints;
mod pipes;
mod streaming;

use clap::{Parser, Subcommand};

use crate::args::{BodySource, HttpArgs, PipeArgs};

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
    command: TopCommand,
}

#[derive(Subcommand, Debug)]
enum TopCommand {
    /// Function-related streaming endpoints.
    Functions {
        #[command(subcommand)]
        command: FunctionsCommand,
    },
}

#[derive(Subcommand, Debug)]
enum FunctionsCommand {
    /// Stream `/functions/executions`. The streaming chunks are
    /// emitted one-per-NDJSON-line on stdout. Per-agent named pipes
    /// appear under `${config_base_dir}/pipes/<agent_id>` for as
    /// long as that agent is in flight; external processes can
    /// connect and write NDJSON `RichContent` lines to push
    /// notifications at the agent.
    Executions {
        #[command(flatten)]
        body: BodySource,
    },
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();
    // Stamp the handle's agent_id from --objectiveai-agent-id so
    // every emitted Notification/Error line carries it — mirrors
    // objectiveai-cli/src/main.rs:16-17.
    let mut handle = objectiveai_sdk::cli::output::Handle::stdout();
    handle.agent_id = cli.http.objectiveai_agent_id.clone();

    match cli.command {
        TopCommand::Functions {
            command: FunctionsCommand::Executions { body },
        } => endpoints::functions_executions::run(&cli.http, &cli.pipes, body, &handle).await,
    }
}
