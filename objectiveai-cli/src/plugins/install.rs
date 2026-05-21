//! `plugins install` subcommand tree.
//!
//! Two variants:
//!
//! - [`Commands::Github`] — the existing GitHub-install flow (fetch
//!   manifest, whitelist-check, download platform binary, write under
//!   `~/.objectiveai/plugins/<repository>/`). Delegates to the
//!   `super::install` fn so the on-disk install pipeline stays
//!   single-sourced.
//! - [`Commands::Filesystem`] — no install action. Emits an
//!   `INSTRUCTIONS.md` notification telling the agent how to author a
//!   plugin locally by hand (manifest schema command path, binary
//!   placement convention, viewer URL-vs-bundle choice).

use clap::Subcommand;
use objectiveai_sdk::cli::output::{Handle, Notification, Output};

#[derive(Subcommand)]
pub enum Commands {
    /// Install a plugin from a GitHub repository. Fetches
    /// `objectiveai.json` from `<owner>/<repository>` at
    /// `--commit-sha` (or the default branch), checks the
    /// (owner, repository, commit_sha_or_HEAD, manifest.version)
    /// quadruple against the install whitelist, then downloads the
    /// matching release asset for this platform and writes it to
    /// `~/.objectiveai/plugins/<repository>/plugin` (with `.exe` on
    /// Windows). Same behaviour as the pre-split flat
    /// `plugins install` command.
    Github(GithubArgs),
    /// Get instructions for authoring a plugin in your local
    /// `~/.objectiveai/plugins/` directory by hand. Takes no args —
    /// the CLI prints an INSTRUCTIONS.md telling the agent the
    /// manifest schema command path, the binary location convention,
    /// and the viewer URL-vs-bundle tradeoff. Nothing is installed.
    Filesystem,
}

#[derive(clap::Args)]
pub struct GithubArgs {
    #[arg(long)]
    pub owner: String,
    #[arg(long)]
    pub repository: String,
    #[arg(long)]
    pub commit_sha: Option<String>,
    /// Bypass the plugin whitelist. Emits a warning notification and
    /// proceeds with the install. Use only if you trust the
    /// repository — a non-whitelisted plugin runs with the same
    /// permissions as any binary on your system.
    #[arg(long)]
    pub allow_untrusted: bool,
    /// Replace an already-installed plugin. Without this flag,
    /// install fails with `AlreadyInstalled` when a manifest exists
    /// at `~/.objectiveai/plugins/<repository>.json`. With it, the
    /// existing binary, viewer/ directory, and sibling manifest are
    /// deleted before the new install runs.
    #[arg(long)]
    pub upgrade: bool,
}

impl Commands {
    pub async fn handle(
        self,
        cli_config: &crate::Config,
        handle: &Handle,
    ) -> Result<(), crate::error::Error> {
        match self {
            Commands::Github(args) => {
                super::install(
                    cli_config,
                    handle,
                    &args.owner,
                    &args.repository,
                    args.commit_sha.as_deref(),
                    args.allow_untrusted,
                    args.upgrade,
                )
                .await
            }
            Commands::Filesystem => emit_instructions(handle).await,
        }
    }
}

#[derive(serde::Serialize)]
struct Instructions {
    instructions: String,
}

async fn emit_instructions(handle: &Handle) -> Result<(), crate::error::Error> {
    let instructions = include_str!(
        "../../assets/plugins/install/filesystem/INSTRUCTIONS.md"
    )
    .to_string();
    Output::<Instructions>::Notification(Notification { agent_id: None,
        value: Instructions { instructions },
    })
    .emit(handle)
    .await;
    Ok(())
}
