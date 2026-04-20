use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get a function invention state by remote path or favorite name
    Get {
        #[command(flatten)]
        args: crate::get::GetArgs,
    },
}

async fn get_favorites(cli_config: &crate::Config) -> Vec<objectiveai::filesystem::config::Favorite> {
    let (_, mut config) = crate::config::read(cli_config).await.unwrap();
    config.functions().get_favorites().to_vec()
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<crate::Output, crate::error::Error> {
        match self {
            Commands::Get { args } => {
                let path = args.resolve(|| get_favorites(cli_config)).await?;
                crate::api::run(|http_client| async move {
                    let response = objectiveai::functions::inventions::state::get_function_invention_state(&http_client, path).await?;
                    Ok(serde_json::to_string(&response).unwrap())
                }, false).await
            }
        }
    }
}
