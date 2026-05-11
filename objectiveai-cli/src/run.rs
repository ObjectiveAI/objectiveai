use clap::{Parser, Subcommand};
use envconfig::Envconfig;

use crate::api;
use crate::agents;
use crate::swarms;
use crate::functions;
use crate::viewer;
use crate::schemas;
use crate::laboratories;
use crate::logs;
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
    /// (see `objectiveai::HttpClient::new` in
    /// `objectiveai-rs/src/http/client.rs`).
    #[envconfig(from = "GITHUB_AUTHORIZATION")]
    github_authorization: Option<String>,
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
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub config_set_forbidden: bool,
    pub config_base_dir: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
    pub github_authorization: Option<String>,
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
}

impl Commands {
    pub async fn handle(self, cli_config: &Config) -> Result<(), error::Error> {
        match self {
            Commands::Api { command } => command.handle(cli_config).await,
            Commands::Agents { command } => command.handle(cli_config).await,
            Commands::Swarms { command } => command.handle(cli_config).await,
            Commands::Functions { command } => command.handle(cli_config).await,
            Commands::Viewer { command } => command.handle(cli_config).await,
            Commands::Schemas { command } => command.handle(),
            Commands::Laboratories { command } => command.handle(cli_config).await,
            Commands::Vector { command } => command.handle(cli_config).await,
            Commands::Logs { command } => command.handle(cli_config).await,
            Commands::Instructions { command } => command.handle(cli_config),
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
/// Returns `Ok(())` when the requested command succeeded — handlers
/// emit their own [`objectiveai_cli_lib::output::Output`] lines inline.
/// Returns `Err(cli_lib::output::Error)` when the command failed; the
/// caller is responsible for emitting that error and choosing an exit
/// code.
pub async fn run<I, T>(
    args: I,
    cli_config: &Config,
) -> Result<(), objectiveai_cli_lib::output::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::try_parse_from(args).map_err(|e| objectiveai_cli_lib::output::Error {
        level: objectiveai_cli_lib::output::Level::Error,
        fatal: true,
        message: e.to_string(),
    })?;
    cli.command
        .handle(cli_config)
        .await
        .map_err(|e| e.to_output(objectiveai_cli_lib::output::Level::Error, true))
}
