//! `tools` subcommand tree — list / get / install / run local-filesystem tools.
//!
//! Mirrors `crate::plugins`'s built-in surface but with two key
//! differences:
//!
//! 1. `tools install` is just instructions (no GitHub fetch); tools
//!    are hand-placed under `~/.objectiveai/tools/`.
//! 2. `tools <name> <args…>` (the external subcommand) spawns the
//!    tool's executable and forwards every stdout/stderr line as a
//!    [`ToolLine`] notification — no JSONL contract. The CLI exits
//!    with the tool's own exit code via [`crate::error::Error::ToolExit`].

use clap::Subcommand;
use objectiveai_sdk::cli::output::{Handle, Notification, Output, Tool, ToolLine, Tools};
use tokio::io::AsyncBufReadExt;

mod install;

#[derive(Subcommand)]
pub enum Commands {
    /// Get a single tool's manifest by name. Emits the manifest as
    /// `{"tool": <manifest>}` when found, or `{"tool": null}` when
    /// the manifest file is missing / unreadable / malformed (same
    /// silent-skip policy as `list`).
    Get {
        /// Tool name (filename stem of the manifest in
        /// `~/.objectiveai/tools/`).
        name: String,
    },
    /// Get instructions for authoring a tool in your local
    /// `~/.objectiveai/tools/` directory by hand. Takes no args —
    /// the CLI prints an INSTRUCTIONS.md telling the agent the
    /// manifest schema command path and the layout convention.
    /// Nothing is installed.
    Install,
    /// List installed tools (every `.json` manifest in
    /// `~/.objectiveai/tools/`). Sorted by manifest mtime, most
    /// recent first. Supports `--offset` / `--limit` for pagination.
    List {
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Run a tool from `~/.objectiveai/tools/`. First element is the
    /// tool name; the rest are forwarded as the tool's argv verbatim.
    /// Stdout and stderr lines are wrapped as `ToolLine` notifications
    /// and emitted. The CLI exits with the tool's exit code.
    #[command(external_subcommand)]
    Run(Vec<String>),
}

impl Commands {
    pub async fn handle(
        self,
        cli_config: &crate::Config,
        handle: &Handle,
    ) -> Result<(), crate::error::Error> {
        match self {
            Commands::Get { name } => get(cli_config, handle, &name).await,
            Commands::Install => install::emit_instructions(handle).await,
            Commands::List { offset, limit } => list(cli_config, handle, offset, limit).await,
            Commands::Run(args) => dispatch_tool(args, cli_config, handle).await,
        }
    }
}

async fn get(
    cli_config: &crate::Config,
    handle: &Handle,
    name: &str,
) -> Result<(), crate::error::Error> {
    let fs_client = crate::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        cli_config.commit_author_name.as_deref(),
        cli_config.commit_author_email.as_deref(),
    );
    let tool = fs_client.get_tool(name).await;
    Output::Notification(Notification {
        value: (Tool { tool }).into(),
    })
    .emit(handle)
    .await;
    Ok(())
}

async fn list(
    cli_config: &crate::Config,
    handle: &Handle,
    offset: usize,
    limit: usize,
) -> Result<(), crate::error::Error> {
    let fs_client = crate::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        cli_config.commit_author_name.as_deref(),
        cli_config.commit_author_email.as_deref(),
    );
    let tools = fs_client.list_tools(offset, limit).await;
    Output::Notification(Notification {
        value: (Tools { tools }).into(),
    })
    .emit(handle)
    .await;
    Ok(())
}

/// Spawn `~/.objectiveai/tools/<exec>` with the trailing args.
/// Forward each stdout line as `ToolLine { stdout: Some(true), … }`
/// and each stderr line as `ToolLine { stderr: Some(true), … }`.
/// Return `Err(ToolExit(code))` for non-zero exits so `run::run`
/// can propagate the code to the CLI process exit.
pub async fn dispatch_tool(
    args: Vec<String>,
    cli_config: &crate::Config,
    handle: &Handle,
) -> Result<(), crate::error::Error> {
    let mut iter = args.into_iter();
    let name = iter
        .next()
        .ok_or(crate::error::Error::MissingArgs("tool name"))?;
    let rest: Vec<String> = iter.collect();

    let fs_client = crate::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        cli_config.commit_author_name.as_deref(),
        cli_config.commit_author_email.as_deref(),
    );
    let exe = match fs_client.resolve_tool(&name).await {
        Some(p) => p,
        None => return Err(crate::error::Error::ToolNotFound(name)),
    };

    let mut cmd = tokio::process::Command::new(&exe);
    cmd.args(&rest)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    // Forward the full cli Config (lineage, MCP session, auth tokens,
    // config_base_dir, …) to the tool subprocess via the same env-var
    // names `EnvConfigBuilder` reads. Inherit-everything-else stays;
    // we only add to the env.
    crate::spawn::apply_config_env(&mut cmd, cli_config);

    let mut child = cmd.spawn().map_err(crate::error::Error::ToolSpawn)?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // Concurrent line-buffered drain on both streams. `tokio::join!`
    // keeps both futures in the same task; no Handle clone required.
    tokio::join!(
        forward_stream(stdout, handle, Stream::Stdout),
        forward_stream(stderr, handle, Stream::Stderr),
    );

    let status = child.wait().await.map_err(crate::error::Error::ToolRead)?;
    if status.success() {
        Ok(())
    } else {
        Err(crate::error::Error::ToolExit(status.code().unwrap_or(1)))
    }
}

enum Stream {
    Stdout,
    Stderr,
}

/// Drain `stream` line-by-line and emit each line as a `ToolLine`
/// notification stamped with the originating stream. EOF and
/// mid-stream I/O errors both terminate the loop quietly — the
/// subprocess exit code is the authoritative failure signal.
async fn forward_stream<R>(stream: R, handle: &Handle, which: Stream)
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut reader = tokio::io::BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\r', '\n']).to_string();
                let value = match which {
                    Stream::Stdout => ToolLine {
                        line: trimmed,
                        stdout: Some(true),
                        stderr: None,
                    },
                    Stream::Stderr => ToolLine {
                        line: trimmed,
                        stdout: None,
                        stderr: Some(true),
                    },
                };
                Output::Notification(Notification {
                    value: value.into(),
                })
                .emit(handle)
                .await;
            }
        }
    }
}
