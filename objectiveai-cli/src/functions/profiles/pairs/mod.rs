pub mod config;
pub mod favorites;

use clap::Subcommand;
use objectiveai_sdk::cli::output::{FunctionProfilePair, Pair};

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

async fn get_favorites(cli_config: &crate::Config) -> Vec<objectiveai_sdk::filesystem::config::PairFavorite> {
    let (_, mut config) = crate::config::read(cli_config).await.unwrap();
    config.functions().profiles().pairs().get_favorites().to_vec()
}

async fn list_objectiveai(
    http_client: objectiveai_sdk::HttpClient,
) -> Result<Vec<objectiveai_sdk::functions::response::ListFunctionProfilePairItem>, crate::error::Error> {
    let response = objectiveai_sdk::functions::list_function_profile_pairs(
        &http_client,
        objectiveai_sdk::functions::request::ListFunctionProfilePairsRequest {
            source: Some(objectiveai_sdk::functions::request::ListFunctionProfilePairsSource::Objectiveai),
        },
    ).await?;
    Ok(response.data)
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Get { args } => {
                let (function_path, profile_path) = args.resolve(|| get_favorites(cli_config)).await?;
                let handle = handle.clone();
                crate::api::run(cli_config, |http_client| async move {
                    let (function, profile) = tokio::join!(
                        objectiveai_sdk::functions::get_function(&http_client, function_path),
                        objectiveai_sdk::functions::profiles::get_profile(&http_client, profile_path),
                    );
                    let pair = FunctionProfilePair {
                        function: function?,
                        profile: profile?,
                    };
                    objectiveai_sdk::cli::output::Output::<Pair>::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: Pair { pair } })
                        .emit(&handle).await;
                    Ok(())
                }).await
            }
            Commands::List { source } => {
                match source {
                    crate::list::Source::Favorites => crate::list::pair_favorites(|| get_favorites(cli_config), handle).await,
                    crate::list::Source::Filesystem => Err(crate::error::Error::PairsSourceNotSupported("filesystem")),
                    crate::list::Source::Objectiveai => crate::list::pair_single(cli_config, |c| Box::pin(list_objectiveai(c)), handle).await,
                    crate::list::Source::Mock => Err(crate::error::Error::PairsSourceNotSupported("mock")),
                    crate::list::Source::All => crate::list::pair_all(
                        cli_config,
                        || get_favorites(cli_config),
                        |c| Box::pin(list_objectiveai(c)),
                        handle,
                    ).await,
                }
            }
            Commands::Config { command } => command.handle(cli_config, handle).await,
            Commands::Favorites { command } => command.handle(cli_config, handle).await,
        }
    }
}
