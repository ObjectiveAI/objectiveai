pub mod config;
pub mod favorites;

use clap::Subcommand;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetFunctionProfilePair {
    pub function: objectiveai::functions::response::GetFunctionResponse,
    pub profile: objectiveai::functions::profiles::response::GetProfileResponse,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Get a function-profile pair by remote paths or favorite name
    Get {
        #[command(flatten)]
        args: crate::get::GetPairArgs,
    },
    /// List function-profile pairs
    List {
        #[command(subcommand)]
        source: crate::list::Source,
    },
    /// Pairs configuration
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
    /// Manage pair favorites
    Favorites {
        #[command(subcommand)]
        command: favorites::Commands,
    },
}

async fn get_favorites(cli_config: &crate::Config) -> Vec<objectiveai::filesystem::config::PairFavorite> {
    let (_, mut config) = crate::config::read(cli_config).await.unwrap();
    config.functions().profiles().pairs().get_favorites().to_vec()
}

async fn list_objectiveai(
    http_client: objectiveai::HttpClient,
) -> Result<Vec<objectiveai::functions::response::ListFunctionProfilePairItem>, crate::error::Error> {
    let response = objectiveai::functions::list_function_profile_pairs(
        &http_client,
        objectiveai::functions::request::ListFunctionProfilePairsRequest {
            source: Some(objectiveai::functions::request::ListFunctionProfilePairsSource::Objectiveai),
        },
    ).await?;
    Ok(response.data)
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        match self {
            Commands::Get { args } => {
                let (function_path, profile_path) = args.resolve(|| get_favorites(cli_config)).await?;
                crate::api::run(|http_client| async move {
                    let (function, profile) = tokio::join!(
                        objectiveai::functions::get_function(&http_client, function_path),
                        objectiveai::functions::profiles::get_profile(&http_client, profile_path),
                    );
                    let pair = GetFunctionProfilePair {
                        function: function?,
                        profile: profile?,
                    };
                    #[derive(serde::Serialize)]
                    struct PairResponse {
                        pair: GetFunctionProfilePair,
                    }
                    objectiveai_cli_lib::output::Output::<PairResponse>::Notification(
                        PairResponse { pair },
                    )
                    .emit();
                    Ok(())
                }, false).await
            }
            Commands::List { source } => {
                match source {
                    crate::list::Source::Favorites => crate::list::pair_favorites(|| get_favorites(cli_config)).await,
                    crate::list::Source::Filesystem => Err(crate::error::Error::PairsSourceNotSupported("filesystem")),
                    crate::list::Source::Objectiveai => crate::list::pair_single(|c| Box::pin(list_objectiveai(c))).await,
                    crate::list::Source::Mock => Err(crate::error::Error::PairsSourceNotSupported("mock")),
                    crate::list::Source::All => crate::list::pair_all(
                        || get_favorites(cli_config),
                        |c| Box::pin(list_objectiveai(c)),
                    ).await,
                }
            }
            Commands::Config { command } => command.handle(cli_config).await,
            Commands::Favorites { command } => command.handle(cli_config).await,
        }
    }
}
