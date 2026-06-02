pub mod completions;
pub mod config;
pub mod favorites;
pub mod list;
pub mod message;
pub mod read;
pub mod spawn;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get an agent by remote path or favorite name
    Get {
        #[command(flatten)]
        args: crate::get::GetArgs,
    },
    /// List agents — `active` (direct children) or `available`
    /// (remote agents by source).
    List {
        #[command(subcommand)]
        command: list::Commands,
    },
    /// Agent completions
    Completions {
        #[command(subcommand)]
        command: completions::Commands,
    },
    /// Agents configuration
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
    /// Manage agent favorites
    Favorites {
        #[command(subcommand)]
        command: favorites::Commands,
    },
    /// Publish an agent to the local filesystem
    Publish {
        /// Repository name
        #[arg(long)]
        repository: String,
        #[command(flatten)]
        body: crate::publish::BodySource,
        #[command(flatten)]
        message: crate::publish::MessageSource,
        /// Overwrite if the file already exists
        #[arg(long)]
        overwrite: bool,
    },
    /// Read queue items
    Read {
        #[command(subcommand)]
        command: read::Commands,
    },
    /// Deliver a message to a running spawned agent (or resume its most
    /// recent completion via continuation if it's dormant)
    Message(message::CommandArgs),
    /// Spawn an agent completion (open a streaming run as a child of this caller)
    Spawn(spawn::CommandArgs),
    /// Return the configured self agent id
    Me,
}

/// Read the user's agent-favorites list from local config. `pub(super)`
/// so the nested `list` submodule can reuse it for `list available
/// favorites` / `list available all` without re-rooting the config
/// read.
pub(super) async fn get_favorites(
    cli_config: &crate::Config,
) -> Vec<crate::filesystem::config::Favorite> {
    let (_, mut config) = crate::config::read(cli_config).await.unwrap();
    config.agents().get_favorites().to_vec()
}

impl Commands {
    pub async fn handle(
        self,
        cli_config: &crate::Config,
        handle: &objectiveai_sdk::cli::output::Handle,
    ) -> Result<(), crate::error::Error> {
        match self {
            Commands::Get { args } => {
                let path = args.resolve(|| get_favorites(cli_config)).await?;
                let handle = handle.clone();
                crate::api::run(cli_config, |http_client| async move {
                    let response = objectiveai_sdk::agent::get_agent(&http_client, path).await?;
                    objectiveai_sdk::cli::output::Output::Notification(
                        objectiveai_sdk::cli::output::Notification {
                            value: (objectiveai_sdk::cli::output::Agent { agent: response }).into(),
                        },
                    )
                    .emit(&handle)
                    .await;
                    Ok(())
                })
                .await
            }
            Commands::List { command } => command.handle(cli_config, handle).await,
            Commands::Completions { command } => command.handle(cli_config, handle).await,
            Commands::Config { command } => command.handle(cli_config, handle).await,
            Commands::Favorites { command } => command.handle(cli_config, handle).await,
            Commands::Read { command } => command.handle(cli_config, handle).await,
            Commands::Message(args) => message::handle(args, cli_config, handle).await,
            Commands::Spawn(args) => spawn::handle(args, cli_config, handle).await,
            Commands::Me => {
                objectiveai_sdk::cli::output::Output::Notification(
                    objectiveai_sdk::cli::output::Notification {
                        value: objectiveai_sdk::cli::output::Me {
                            agent_instance_hierarchy: cli_config.agent_instance_hierarchy.clone(),
                        }
                        .into(),
                    },
                )
                .emit(handle)
                .await;
                Ok(())
            }
            Commands::Publish {
                repository,
                body,
                message,
                overwrite,
            } => {
                let agent: objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks = body.resolve()?;
                let msg = message.resolve()?;
                let fs_client = crate::filesystem::Client::new(
                    cli_config.config_base_dir.as_deref(),
                    cli_config.commit_author_name.as_deref(),
                    cli_config.commit_author_email.as_deref(),
                );
                let sha = crate::filesystem::publish::publish_agent(
                    &fs_client,
                    &repository,
                    &agent,
                    &msg,
                    overwrite,
                )
                .await?;
                objectiveai_sdk::cli::output::Output::Notification(
                    objectiveai_sdk::cli::output::Notification {
                        value: (objectiveai_sdk::cli::output::Published { sha }).into(),
                    },
                )
                .emit(handle)
                .await;
                Ok(())
            }
        }
    }
}
