pub mod config;
pub mod favorites;
pub mod inventions;
pub mod profiles;
pub mod executions;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get a function by remote path or favorite name
    Get {
        #[command(flatten)]
        args: crate::get::GetArgs,
    },
    /// List functions
    List {
        #[command(subcommand)]
        source: crate::list::Source,
    },
    /// Functions configuration
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
    /// Manage function favorites
    Favorites {
        #[command(subcommand)]
        command: favorites::Commands,
    },
    /// Function executions
    Executions {
        #[command(subcommand)]
        command: executions::Commands,
    },
    /// Functions inventions
    Inventions {
        #[command(subcommand)]
        command: inventions::Commands,
    },
    /// Functions profiles
    Profiles {
        #[command(subcommand)]
        command: profiles::Commands,
    },
    /// Publish a function to the local filesystem
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
    config.functions().get_favorites().to_vec()
}

async fn list_source(
    http_client: objectiveai::HttpClient,
    source: objectiveai::functions::request::ListFunctionsSource,
) -> Result<Vec<objectiveai::RemotePath>, crate::error::Error> {
    let response = objectiveai::functions::list_functions(
        &http_client,
        objectiveai::functions::request::ListFunctionsRequest { source: Some(source) },
    ).await?;
    Ok(response.data)
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        match self {
            Commands::Get { args } => {
                let path = args.resolve(|| get_favorites(cli_config)).await?;
                crate::api::run(|http_client| async move {
                    let response = objectiveai::functions::get_function(&http_client, path).await?;
                    #[derive(serde::Serialize)]
                    struct FunctionResponse {
                        function: objectiveai::functions::response::GetFunctionResponse,
                    }
                    objectiveai_cli_lib::output::Output::<FunctionResponse>::Notification(
                        FunctionResponse { function: response },
                    )
                    .emit();
                    Ok(())
                }, false).await
            }
            Commands::List { source } => {
                use objectiveai::functions::request::ListFunctionsSource;
                match source {
                    crate::list::Source::Favorites => crate::list::favorites(|| get_favorites(cli_config)).await,
                    crate::list::Source::Filesystem => crate::list::single(|c| Box::pin(list_source(c, ListFunctionsSource::Filesystem))).await,
                    crate::list::Source::Objectiveai => crate::list::single(|c| Box::pin(list_source(c, ListFunctionsSource::Objectiveai))).await,
                    crate::list::Source::Mock => crate::list::single(|c| Box::pin(list_source(c, ListFunctionsSource::Mock))).await,
                    crate::list::Source::All => crate::list::all(
                        || get_favorites(cli_config),
                        |c| Box::pin(list_source(c, ListFunctionsSource::Filesystem)),
                        |c| Box::pin(list_source(c, ListFunctionsSource::Objectiveai)),
                    ).await,
                }
            }
            Commands::Executions { command } => command.handle(cli_config).await,
            Commands::Config { command } => command.handle(cli_config).await,
            Commands::Favorites { command } => command.handle(cli_config).await,
            Commands::Inventions { command } => command.handle(cli_config).await,
            Commands::Profiles { command } => command.handle(cli_config).await,
            Commands::Publish { repository, body, message, overwrite } => {
                let function: objectiveai::functions::FullRemoteFunction = body.resolve()?;
                let msg = message.resolve()?;
                let fs_client = objectiveai::filesystem::Client::new(
                    cli_config.config_base_dir.as_deref(),
                    cli_config.commit_author_name.as_deref(),
                    cli_config.commit_author_email.as_deref(),
                );
                let sha = objectiveai::filesystem::publish::publish_function(
                    &fs_client, &repository, &function, &msg, overwrite,
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
