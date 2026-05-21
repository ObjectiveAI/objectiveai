use clap::{Parser, Subcommand};
use envconfig::Envconfig;

use crate::api;
use crate::agents;
use crate::swarms;
use crate::functions;
use crate::viewer;
use crate::mcp;
use crate::schemas;
use crate::laboratories;
use crate::logs;
use crate::plugins;
use crate::vector;
use crate::instructions;
use crate::error;

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
    #[envconfig(from = "OBJECTIVEAI_AGENT_ID")]
    agent_id: Option<String>,
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
            agent_id: self.agent_id,
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
    pub agent_id: Option<String>,
}

impl Envconfig for ConfigBuilder {
    #[allow(deprecated)]
    fn init() -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init().map(|e| e.build())
    }

    fn init_from_env() -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init_from_env().map(|e| e.build())
    }

    fn init_from_hashmap(hashmap: &std::collections::HashMap<String, String>) -> Result<Self, envconfig::Error> {
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
            agent_id: self.agent_id,
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
    pub agent_id: Option<String>,
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
    /// API configuration and operations
    Api {
        #[command(subcommand)]
        command: api::Commands,
    },
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
    /// Browse JSON schemas
    Schemas {
        #[command(subcommand)]
        command: schemas::Commands,
    },
    /// Laboratories management
    Laboratories {
        #[command(subcommand)]
        command: laboratories::Commands,
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
    /// Manage Instructions IDs across all streaming `create` scopes
    Instructions {
        #[command(subcommand)]
        command: instructions::Commands,
    },
    /// Plugins management
    Plugins {
        #[command(subcommand)]
        command: plugins::Commands,
    },
    /// Update the cli and all managed binaries (api, viewer, mcp) from
    /// the latest GitHub release. Refuses to proceed unless all four
    /// expected assets are present for the host triple.
    Update,
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
            Commands::Api { command } => command.handle(cli_config, handle).await,
            Commands::Agents { command } => command.handle(cli_config, handle).await,
            Commands::Swarms { command } => command.handle(cli_config, handle).await,
            Commands::Functions { command } => command.handle(cli_config, handle).await,
            Commands::Viewer { command } => command.handle(cli_config, handle).await,
            Commands::Mcp { command } => command.handle(cli_config, handle).await,
            Commands::Schemas { command } => command.handle(handle).await,
            Commands::Laboratories { command } => command.handle(cli_config, handle).await,
            Commands::Vector { command } => command.handle(cli_config, handle).await,
            Commands::Logs { command } => command.handle(cli_config, handle).await,
            Commands::Instructions { command } => command.handle(cli_config, handle).await,
            Commands::Plugins { command } => command.handle(cli_config, handle).await,
            Commands::Update => crate::updater::run_update(cli_config, handle).await,
            Commands::External(args) => crate::plugins::dispatch_external(args, cli_config, handle).await,
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
    use objectiveai_sdk::cli::output::{Error as OutputError, Level, Notification, Output};

    Output::<serde_json::Value>::Begin.emit(&handle).await;

    let code = match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command.handle(cli_config, &handle).await {
            Ok(()) => 0,
            Err(e) => {
                let err = e.to_output(Level::Error, true);
                Output::<serde_json::Value>::Error(err).emit(&handle).await;
                1
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
            Output::<String>::Notification(Notification { agent_id: None,
                value: e.to_string(),
            })
            .emit(&handle)
            .await;
            0
        }
        Err(e) => {
            let err = OutputError {
                level: Level::Error,
                fatal: true,
                message: e.to_string().into(),
                agent_id: None,
            };
            Output::<serde_json::Value>::Error(err).emit(&handle).await;
            1
        }
    };

    Output::<serde_json::Value>::End.emit(&handle).await;
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
