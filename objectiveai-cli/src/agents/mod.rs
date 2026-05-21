pub mod completions;
pub mod config;
pub mod favorites;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get an agent by remote path or favorite name
    Get {
        #[command(flatten)]
        args: crate::get::GetArgs,
    },
    /// List agents
    List {
        #[command(subcommand)]
        source: crate::list::Source,
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
}

async fn get_favorites(cli_config: &crate::Config) -> Vec<objectiveai_sdk::filesystem::config::Favorite> {
    let (_, mut config) = crate::config::read(cli_config).await.unwrap();
    config.agents().get_favorites().to_vec()
}

async fn list_source(
    http_client: objectiveai_sdk::HttpClient,
    source: objectiveai_sdk::agent::request::ListAgentsSource,
) -> Result<Vec<objectiveai_sdk::RemotePath>, crate::error::Error> {
    let response = objectiveai_sdk::agent::list_agents(
        &http_client,
        objectiveai_sdk::agent::request::ListAgentsRequest { source: Some(source) },
    ).await?;
    Ok(response.data)
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Get { args } => {
                let path = args.resolve(|| get_favorites(cli_config)).await?;
                let handle = handle.clone();
                crate::api::run(|http_client| async move {
                    let response = objectiveai_sdk::agent::get_agent(&http_client, path).await?;
                    objectiveai_sdk::cli::output::Output::<objectiveai_sdk::cli::output::Agent>::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: 
                        objectiveai_sdk::cli::output::Agent { agent: response },
                     })
                    .emit(&handle).await;
                    Ok(())
                }).await
            }
            Commands::List { source } => {
                use objectiveai_sdk::agent::request::ListAgentsSource;
                match source {
                    crate::list::Source::Favorites => crate::list::favorites(|| get_favorites(cli_config), handle).await,
                    crate::list::Source::Filesystem => crate::list::single(|c| Box::pin(list_source(c, ListAgentsSource::Filesystem)), handle).await,
                    crate::list::Source::Objectiveai => crate::list::single(|c| Box::pin(list_source(c, ListAgentsSource::Objectiveai)), handle).await,
                    crate::list::Source::Mock => crate::list::single(|c| Box::pin(list_source(c, ListAgentsSource::Mock)), handle).await,
                    crate::list::Source::All => crate::list::all(
                        || get_favorites(cli_config),
                        |c| Box::pin(list_source(c, ListAgentsSource::Filesystem)),
                        |c| Box::pin(list_source(c, ListAgentsSource::Objectiveai)),
                        handle,
                    ).await,
                }
            }
            Commands::Completions { command } => command.handle(cli_config, handle).await,
            Commands::Config { command } => command.handle(cli_config, handle).await,
            Commands::Favorites { command } => command.handle(cli_config, handle).await,
            Commands::Publish { repository, body, message, overwrite } => {
                let agent: objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks = body.resolve()?;
                let msg = message.resolve()?;
                let fs_client = objectiveai_sdk::filesystem::Client::new(
                    cli_config.config_base_dir.as_deref(),
                    cli_config.commit_author_name.as_deref(),
                    cli_config.commit_author_email.as_deref(),
                );
                let sha = objectiveai_sdk::filesystem::publish::publish_agent(
                    &fs_client, &repository, &agent, &msg, overwrite,
                ).await?;
                objectiveai_sdk::cli::output::Output::<objectiveai_sdk::cli::output::Published>::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: 
                    objectiveai_sdk::cli::output::Published { sha },
                 })
                .emit(handle).await;
                Ok(())
            }
        }
    }
}
