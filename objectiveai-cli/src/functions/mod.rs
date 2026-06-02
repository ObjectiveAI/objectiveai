pub mod config;
pub mod executions;
pub mod favorites;
pub mod inventions;
pub mod profiles;

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

async fn get_favorites(
    cli_config: &crate::Config,
) -> Vec<crate::filesystem::config::Favorite> {
    let (_, mut config) = crate::config::read(cli_config).await.unwrap();
    config.functions().get_favorites().to_vec()
}

async fn list_source(
    http_client: objectiveai_sdk::HttpClient,
    source: objectiveai_sdk::functions::request::ListFunctionsSource,
) -> Result<Vec<objectiveai_sdk::RemotePath>, crate::error::Error> {
    let response = objectiveai_sdk::functions::list_functions(
        &http_client,
        objectiveai_sdk::functions::request::ListFunctionsRequest {
            source: Some(source),
        },
    )
    .await?;
    Ok(response.data)
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
                    let response =
                        objectiveai_sdk::functions::get_function(&http_client, path).await?;
                    objectiveai_sdk::cli::output::Output::Notification(
                        objectiveai_sdk::cli::output::Notification {
                            value: (objectiveai_sdk::cli::output::Function { function: response })
                                .into(),
                        },
                    )
                    .emit(&handle)
                    .await;
                    Ok(())
                })
                .await
            }
            Commands::List { source } => {
                use objectiveai_sdk::functions::request::ListFunctionsSource;
                match source {
                    crate::list::Source::Favorites => {
                        crate::list::favorites(|| get_favorites(cli_config), handle).await
                    }
                    crate::list::Source::Filesystem => {
                        crate::list::single(
                            cli_config,
                            |c| Box::pin(list_source(c, ListFunctionsSource::Filesystem)),
                            handle,
                        )
                        .await
                    }
                    crate::list::Source::Objectiveai => {
                        crate::list::single(
                            cli_config,
                            |c| Box::pin(list_source(c, ListFunctionsSource::Objectiveai)),
                            handle,
                        )
                        .await
                    }
                    crate::list::Source::Mock => {
                        crate::list::single(
                            cli_config,
                            |c| Box::pin(list_source(c, ListFunctionsSource::Mock)),
                            handle,
                        )
                        .await
                    }
                    crate::list::Source::All => {
                        crate::list::all(
                            cli_config,
                            || get_favorites(cli_config),
                            |c| Box::pin(list_source(c, ListFunctionsSource::Filesystem)),
                            |c| Box::pin(list_source(c, ListFunctionsSource::Objectiveai)),
                            handle,
                        )
                        .await
                    }
                }
            }
            Commands::Executions { command } => command.handle(cli_config, handle).await,
            Commands::Config { command } => command.handle(cli_config, handle).await,
            Commands::Favorites { command } => command.handle(cli_config, handle).await,
            Commands::Inventions { command } => command.handle(cli_config, handle).await,
            Commands::Profiles { command } => command.handle(cli_config, handle).await,
            Commands::Publish {
                repository,
                body,
                message,
                overwrite,
            } => {
                let function: objectiveai_sdk::functions::FullRemoteFunction = body.resolve()?;
                let msg = message.resolve()?;
                let fs_client = crate::filesystem::Client::new(
                    cli_config.config_base_dir.as_deref(),
                    cli_config.commit_author_name.as_deref(),
                    cli_config.commit_author_email.as_deref(),
                );
                let sha = crate::filesystem::publish::publish_function(
                    &fs_client,
                    &repository,
                    &function,
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
