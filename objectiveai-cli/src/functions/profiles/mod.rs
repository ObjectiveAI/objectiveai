pub mod config;
pub mod favorites;
pub mod pairs;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get a profile by remote path or favorite name
    Get {
        #[command(flatten)]
        args: crate::get::GetArgs,
    },
    /// List profiles
    List {
        #[command(subcommand)]
        source: crate::list::Source,
    },
    /// Profiles configuration
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
    /// Manage profile favorites
    Favorites {
        #[command(subcommand)]
        command: favorites::Commands,
    },
    /// Function-profile pairs
    Pairs {
        #[command(subcommand)]
        command: pairs::Commands,
    },
    /// Publish a profile to the local filesystem
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
    config.functions().profiles().get_favorites().to_vec()
}

async fn list_source(
    http_client: objectiveai::HttpClient,
    source: objectiveai::functions::profiles::request::ListProfilesSource,
) -> Result<Vec<objectiveai::RemotePath>, crate::error::Error> {
    let response = objectiveai::functions::profiles::list_profiles(
        &http_client,
        objectiveai::functions::profiles::request::ListProfilesRequest { source: Some(source) },
    ).await?;
    Ok(response.data)
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        match self {
            Commands::Get { args } => {
                let path = args.resolve(|| get_favorites(cli_config)).await?;
                crate::api::run(|http_client| async move {
                    let response = objectiveai::functions::profiles::get_profile(&http_client, path).await?;
                    #[derive(serde::Serialize)]
                    struct ProfileResponse {
                        profile: objectiveai::functions::profiles::response::GetProfileResponse,
                    }
                    objectiveai_cli_lib::output::Output::<ProfileResponse>::Notification(
                        ProfileResponse { profile: response },
                    )
                    .emit();
                    Ok(())
                }, false).await
            }
            Commands::List { source } => {
                use objectiveai::functions::profiles::request::ListProfilesSource;
                match source {
                    crate::list::Source::Favorites => crate::list::favorites(|| get_favorites(cli_config)).await,
                    crate::list::Source::Filesystem => crate::list::single(|c| Box::pin(list_source(c, ListProfilesSource::Filesystem))).await,
                    crate::list::Source::Objectiveai => crate::list::single(|c| Box::pin(list_source(c, ListProfilesSource::Objectiveai))).await,
                    crate::list::Source::Mock => crate::list::single(|c| Box::pin(list_source(c, ListProfilesSource::Mock))).await,
                    crate::list::Source::All => crate::list::all(
                        || get_favorites(cli_config),
                        |c| Box::pin(list_source(c, ListProfilesSource::Filesystem)),
                        |c| Box::pin(list_source(c, ListProfilesSource::Objectiveai)),
                    ).await,
                }
            }
            Commands::Config { command } => command.handle(cli_config).await,
            Commands::Favorites { command } => command.handle(cli_config).await,
            Commands::Pairs { command } => command.handle(cli_config).await,
            Commands::Publish { repository, body, message, overwrite } => {
                let profile: objectiveai::functions::RemoteProfile = body.resolve()?;
                let msg = message.resolve()?;
                let fs_client = objectiveai::filesystem::Client::new(
                    cli_config.config_base_dir.as_deref(),
                    cli_config.commit_author_name.as_deref(),
                    cli_config.commit_author_email.as_deref(),
                );
                let sha = objectiveai::filesystem::publish::publish_profile(
                    &fs_client, &repository, &profile, &msg, overwrite,
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
