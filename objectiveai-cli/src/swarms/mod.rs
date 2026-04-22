pub mod config;
pub mod favorites;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get a swarm by remote path or favorite name
    Get {
        #[command(flatten)]
        args: crate::get::GetArgs,
    },
    /// List swarms
    List {
        #[command(subcommand)]
        source: crate::list::Source,
    },
    /// Swarms configuration
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
    /// Manage swarm favorites
    Favorites {
        #[command(subcommand)]
        command: favorites::Commands,
    },
    /// Publish a swarm to the local filesystem
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
    config.swarms().get_favorites().to_vec()
}

async fn list_source(
    http_client: objectiveai::HttpClient,
    source: objectiveai::swarm::request::ListSwarmsSource,
) -> Result<Vec<objectiveai::RemotePath>, crate::error::Error> {
    let response = objectiveai::swarm::list_swarms(
        &http_client,
        objectiveai::swarm::request::ListSwarmsRequest { source: Some(source) },
    ).await?;
    Ok(response.data)
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<crate::Output, crate::error::Error> {
        match self {
            Commands::Get { args } => {
                let path = args.resolve(|| get_favorites(cli_config)).await?;
                crate::api::run(|http_client| async move {
                    let response = objectiveai::swarm::get_swarm(&http_client, path).await?;
                    Ok(serde_json::to_string(&response).unwrap())
                }, false).await
            }
            Commands::List { source } => {
                use objectiveai::swarm::request::ListSwarmsSource;
                match source {
                    crate::list::Source::Favorites => crate::list::favorites(|| get_favorites(cli_config)).await,
                    crate::list::Source::Filesystem => crate::list::single(|c| Box::pin(list_source(c, ListSwarmsSource::Filesystem))).await,
                    crate::list::Source::Objectiveai => crate::list::single(|c| Box::pin(list_source(c, ListSwarmsSource::Objectiveai))).await,
                    crate::list::Source::Mock => crate::list::single(|c| Box::pin(list_source(c, ListSwarmsSource::Mock))).await,
                    crate::list::Source::All => crate::list::all(
                        || get_favorites(cli_config),
                        |c| Box::pin(list_source(c, ListSwarmsSource::Filesystem)),
                        |c| Box::pin(list_source(c, ListSwarmsSource::Objectiveai)),
                    ).await,
                }
            }
            Commands::Config { command } => command.handle(cli_config).await,
            Commands::Favorites { command } => command.handle(cli_config).await,
            Commands::Publish { repository, body, message, overwrite } => {
                let swarm: objectiveai::swarm::RemoteSwarmBase = body.resolve()?;
                let msg = message.resolve()?;
                let fs_client = objectiveai::filesystem::Client::new(
                    cli_config.config_base_dir.as_deref(),
                    cli_config.commit_author_name.as_deref(),
                    cli_config.commit_author_email.as_deref(),
                );
                let sha = objectiveai::filesystem::publish::publish_swarm(
                    &fs_client, &repository, &swarm, &msg, overwrite,
                ).await?;
                Ok(crate::Output::Api(sha))
            }
        }
    }
}
