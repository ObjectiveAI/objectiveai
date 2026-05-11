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

async fn get_favorites(cli_config: &crate::Config) -> Vec<objectiveai::filesystem::config::Favorite> {
    let (_, mut config) = crate::config::read(cli_config).await.unwrap();
    config.agents().get_favorites().to_vec()
}

async fn list_source(
    http_client: objectiveai::HttpClient,
    source: objectiveai::agent::request::ListAgentsSource,
) -> Result<Vec<objectiveai::RemotePath>, crate::error::Error> {
    let response = objectiveai::agent::list_agents(
        &http_client,
        objectiveai::agent::request::ListAgentsRequest { source: Some(source) },
    ).await?;
    Ok(response.data)
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        match self {
            Commands::Get { args } => {
                let path = args.resolve(|| get_favorites(cli_config)).await?;
                crate::api::run(|http_client| async move {
                    let response = objectiveai::agent::get_agent(&http_client, path).await?;
                    #[derive(serde::Serialize)]
                    struct AgentResponse {
                        agent: objectiveai::agent::response::GetAgentResponse,
                    }
                    objectiveai_cli_lib::output::Output::<AgentResponse>::Notification(
                        AgentResponse { agent: response },
                    )
                    .emit();
                    Ok(())
                }, false).await
            }
            Commands::List { source } => {
                use objectiveai::agent::request::ListAgentsSource;
                match source {
                    crate::list::Source::Favorites => crate::list::favorites(|| get_favorites(cli_config)).await,
                    crate::list::Source::Filesystem => crate::list::single(|c| Box::pin(list_source(c, ListAgentsSource::Filesystem))).await,
                    crate::list::Source::Objectiveai => crate::list::single(|c| Box::pin(list_source(c, ListAgentsSource::Objectiveai))).await,
                    crate::list::Source::Mock => crate::list::single(|c| Box::pin(list_source(c, ListAgentsSource::Mock))).await,
                    crate::list::Source::All => crate::list::all(
                        || get_favorites(cli_config),
                        |c| Box::pin(list_source(c, ListAgentsSource::Filesystem)),
                        |c| Box::pin(list_source(c, ListAgentsSource::Objectiveai)),
                    ).await,
                }
            }
            Commands::Completions { command } => command.handle(cli_config).await,
            Commands::Config { command } => command.handle(cli_config).await,
            Commands::Favorites { command } => command.handle(cli_config).await,
            Commands::Publish { repository, body, message, overwrite } => {
                let agent: objectiveai::agent::RemoteAgentBaseWithFallbacks = body.resolve()?;
                let msg = message.resolve()?;
                let fs_client = objectiveai::filesystem::Client::new(
                    cli_config.config_base_dir.as_deref(),
                    cli_config.commit_author_name.as_deref(),
                    cli_config.commit_author_email.as_deref(),
                );
                let sha = objectiveai::filesystem::publish::publish_agent(
                    &fs_client, &repository, &agent, &msg, overwrite,
                ).await?;
                #[derive(serde::Serialize)]
                struct Published {
                    sha: String,
                }
                objectiveai_cli_lib::output::Output::<Published>::Notification(
                    Published { sha },
                )
                .emit();
                Ok(())
            }
        }
    }
}
