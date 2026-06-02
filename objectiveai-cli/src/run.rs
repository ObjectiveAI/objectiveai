use clap::{Parser, Subcommand};
use envconfig::Envconfig;

use crate::agents;
use crate::error;
use crate::functions;
use crate::instance;
use crate::logs;
use crate::mcp;
use crate::plugins;
use crate::swarms;
use crate::tools;
use crate::vector;
use crate::viewer;

#[derive(Envconfig)]
struct EnvConfigBuilder {
    #[envconfig(from = "CONFIG_SET_FORBIDDEN")]
    config_set_forbidden: Option<String>,
    #[envconfig(from = "CONFIG_BASE_DIR")]
    config_base_dir: Option<String>,
    #[envconfig(from = "COMMIT_AUTHOR_NAME")]
    commit_author_name: Option<String>,
    #[envconfig(from = "COMMIT_AUTHOR_EMAIL")]
    commit_author_email: Option<String>,
    /// Consumed by the auto-updater to authenticate against GitHub's
    /// release API — matches the env name the rest of the CLI honours
    /// (see `objectiveai_sdk::HttpClient::new` in
    /// `objectiveai-rs/src/http/client.rs`).
    #[envconfig(from = "GITHUB_AUTHORIZATION")]
    github_authorization: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY")]
    agent_instance_hierarchy: Option<String>,
    /// Bare 22-character agent_id of the agent invoking this cli. The
    /// full `agent_instance_hierarchy` is this id plus the agent-
    /// completion that spawned it plus the parent-agent hierarchy;
    /// `agent_id` is just the leaf identity, without the lineage.
    /// `None` when the cli isn't being invoked by an agent.
    #[envconfig(from = "OBJECTIVEAI_AGENT_ID")]
    agent_id: Option<String>,
    /// MCP session id, propagated by `objectiveai-mcp` when it
    /// invokes this cli to run a tool, and by the cli further
    /// into every tool subprocess it spawns. Same string as the
    /// `Mcp-Session-Id` HTTP header.
    #[envconfig(from = "MCP_SESSION_ID")]
    mcp_session_id: Option<String>,
    /// Boolean marker set by `objectiveai-mcp` at startup. When true,
    /// every plugin / tool subprocess this cli spawns also gets the
    /// flag in its env so descendants can tell they're under MCP.
    /// Constant: `objectiveai_sdk::mcp::OBJECTIVEAI_MCP_ENV`.
    #[envconfig(from = "OBJECTIVEAI_MCP")]
    mcp: Option<String>,
}

impl EnvConfigBuilder {
    pub fn build(self) -> ConfigBuilder {
        fn parse_bool(s: &str) -> bool {
            let v = s.trim();
            !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false")
        }
        ConfigBuilder {
            config_set_forbidden: self.config_set_forbidden.map(|s| parse_bool(&s)),
            config_base_dir: self.config_base_dir,
            commit_author_name: self.commit_author_name,
            commit_author_email: self.commit_author_email,
            github_authorization: self.github_authorization,
            agent_instance_hierarchy: self.agent_instance_hierarchy,
            agent_id: self.agent_id,
            mcp_session_id: self.mcp_session_id,
            mcp: self.mcp.map(|s| parse_bool(&s)),
        }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    pub config_set_forbidden: Option<bool>,
    pub config_base_dir: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
    pub github_authorization: Option<String>,
    pub agent_instance_hierarchy: Option<String>,
    pub agent_id: Option<String>,
    pub mcp_session_id: Option<String>,
    pub mcp: Option<bool>,
}

impl Envconfig for ConfigBuilder {
    #[allow(deprecated)]
    fn init() -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init().map(|e| e.build())
    }

    fn init_from_env() -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init_from_env().map(|e| e.build())
    }

    fn init_from_hashmap(
        hashmap: &std::collections::HashMap<String, String>,
    ) -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init_from_hashmap(hashmap).map(|e| e.build())
    }
}

impl ConfigBuilder {
    pub fn build(self) -> Config {
        Config {
            config_set_forbidden: self.config_set_forbidden.unwrap_or(false),
            config_base_dir: self.config_base_dir,
            commit_author_name: self.commit_author_name,
            commit_author_email: self.commit_author_email,
            github_authorization: self.github_authorization,
            agent_instance_hierarchy: self.agent_instance_hierarchy.unwrap_or_else(|| "cli".to_string()),
            agent_id: self.agent_id,
            mcp_session_id: self.mcp_session_id,
            mcp: self.mcp.unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub config_set_forbidden: bool,
    pub config_base_dir: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
    pub github_authorization: Option<String>,
    /// Always populated. Defaults to `"cli"` for direct CLI invocations
    /// (the `ConfigBuilder` fills it in if `OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY` is
    /// unset). Programmatic embedders that build a `Config` directly
    /// (e.g. `objectiveai-mcp`) supply their own default — `"mcp"`
    /// for MCP-routed calls, then per-request overridden from the
    /// `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` header.
    pub agent_instance_hierarchy: String,
    /// Bare 22-character agent_id of the agent invoking this cli,
    /// without the agent-completion that spawned it or the parent-
    /// agent hierarchy. The `agent_instance_hierarchy` above is this
    /// id glued onto the lineage. Sourced from `OBJECTIVEAI_AGENT_ID`;
    /// `None` when the cli isn't being invoked by an agent.
    pub agent_id: Option<String>,
    pub mcp_session_id: Option<String>,
    pub mcp: bool,
}

#[derive(Parser)]
#[command(name = "objectiveai")]
#[command(about = "ObjectiveAI CLI")]
#[command(after_help = "\
JSON schemas for every public type are available via `objectiveai schemas`. \
Run `objectiveai schemas --help` to browse them, or pipe a specific schema \
into your tool of choice to drive structured-output generation.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Agents management
    Agents {
        #[command(subcommand)]
        command: agents::Commands,
    },
    /// Swarms management
    Swarms {
        #[command(subcommand)]
        command: swarms::Commands,
    },
    /// Functions management
    Functions {
        #[command(subcommand)]
        command: functions::Commands,
    },
    /// Viewer management
    Viewer {
        #[command(subcommand)]
        command: viewer::Commands,
    },
    /// MCP server management
    Mcp {
        #[command(subcommand)]
        command: mcp::Commands,
    },
    /// Vector completions
    Vector {
        #[command(subcommand)]
        command: vector::Commands,
    },
    /// Browse and read logs
    Logs {
        #[command(subcommand)]
        command: logs::Commands,
    },
    /// Plugins management
    Plugins {
        #[command(subcommand)]
        command: plugins::Commands,
    },
    /// Local-filesystem tools — list, get, install (instructions only).
    Tools {
        #[command(subcommand)]
        command: tools::Commands,
    },
    /// Update the cli and all managed binaries (api, viewer, mcp) from
    /// the latest GitHub release. Refuses to proceed unless all four
    /// expected assets are present for the host triple.
    Update,
    /// Per-agent-instance subprocess runner. Spawned internally by
    /// streaming endpoints (`agents spawn`, `functions executions
    /// create`, …) as `objectiveai-cli instance`. All inputs ride
    /// over an inherited anonymous-pipe handshake; the subcommand
    /// rejects direct shell invocation. Hidden from `--help`.
    #[command(hide = true)]
    Instance,
    /// Run a plugin from `~/.objectiveai/plugins/`. First element is
    /// the plugin name; the rest are forwarded as the plugin's argv.
    /// Captured via clap's external-subcommand mechanism — any first
    /// arg that isn't a known built-in lands here.
    #[command(external_subcommand)]
    External(Vec<String>),
}

impl Commands {
    pub async fn handle(
        self,
        cli_config: &Config,
        handle: &objectiveai_sdk::cli::output::Handle,
    ) -> Result<(), error::Error> {
        match self {
            Commands::Agents { command } => command.handle(cli_config, handle).await,
            Commands::Swarms { command } => command.handle(cli_config, handle).await,
            Commands::Functions { command } => command.handle(cli_config, handle).await,
            Commands::Viewer { command } => command.handle(cli_config, handle).await,
            Commands::Mcp { command } => command.handle(cli_config, handle).await,
            Commands::Vector { command } => command.handle(cli_config, handle).await,
            Commands::Logs { command } => command.handle(cli_config, handle).await,
            Commands::Plugins { command } => command.handle(cli_config, handle).await,
            Commands::Tools { command } => command.handle(cli_config, handle).await,
            Commands::Update => crate::updater::run_update(cli_config, handle).await,
            Commands::Instance => crate::instance::run(handle)
                .await
                .map_err(error::Error::Instance),
            Commands::External(args) => {
                crate::plugins::dispatch_external(args, cli_config, handle).await
            }
        }
    }
}

/// Build the top-level CLI config from the process environment.
///
/// Isolated so `main.rs` can construct a single `Config` and share it
/// with both the auto-updater (before parsing argv) and `run` itself.
pub fn load_config() -> Config {
    ConfigBuilder::init_from_env().unwrap_or_default().build()
}

/// Run the CLI, parsing arguments from the provided iterator.
/// The iterator should include the binary name as the first element
/// (e.g., `["objectiveai", "agents", "list"]`).
///
/// Emits [`objectiveai_sdk::cli::output::BEGIN`] as the very first line
/// and [`objectiveai_sdk::cli::output::END`] as the very last line.
/// Everything else (handler output, error notifications) appears
/// between those bookends.
///
/// Returns an exit code: `0` on success, `1` on any failure. Errors
/// are emitted internally as `Output::Error` via `handle` before the
/// function returns — the caller doesn't see them. Handlers emit
/// their own [`objectiveai_sdk::cli::output::Output`] lines for
/// successful runs.
pub async fn run<I, T>(
    args: I,
    cli_config: &Config,
    handle: objectiveai_sdk::cli::output::Handle,
) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    use objectiveai_sdk::cli::output::{
        Error as OutputError, ErrorType, Level, Notification, Output,
    };

    let code = match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command.handle(cli_config, &handle).await {
            Ok(()) => 0,
            Err(e) => {
                let exit_code = match &e {
                    error::Error::ToolExit(code) => *code,
                    _ => 1,
                };
                let err = e.to_output(Level::Error, true);
                Output::Error(err).emit(&handle).await;
                exit_code
            }
        },
        Err(e) if is_informational(&e) => {
            // --help, --version, or "no subcommand → show help". Not
            // an error — render as a single notification with the help
            // text as a plain string. Exit 0 so scripts can run
            // `objectiveai --help` without `&&` short-circuiting.
            //
            // Going through Output::Error here would also cause a
            // stderr-mirror double-print (handle.rs emits fatal errors
            // to both stdout AND stderr so they survive stdout
            // capture), which is wrong for help output.
            Output::Notification(Notification {
                value: objectiveai_sdk::cli::output::Help {
                    help: e.to_string(),
                }
                .into(),
            })
            .emit(&handle)
            .await;
            0
        }
        Err(e) => {
            let err = OutputError {
                r#type: ErrorType::Error,
                level: Level::Error,
                fatal: true,
                message: e.to_string().into(),
                agent_instance_hierarchy: None,
            };
            Output::Error(err).emit(&handle).await;
            1
        }
    };

    code
}

/// Did clap exit with one of the "successful informational output"
/// variants? `--help`, `--version`, or a missing-subcommand bail. These
/// aren't errors — they're rendered as Notifications rather than
/// fatal Errors so consumers don't treat them as failures and the
/// stderr-mirror dance doesn't double-print to mixed terminals.
fn is_informational(e: &clap::Error) -> bool {
    use clap::error::ErrorKind;
    matches!(
        e.kind(),
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    )
}
