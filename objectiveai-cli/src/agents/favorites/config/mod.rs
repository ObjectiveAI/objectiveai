use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get all favorites
    Get,
    /// Add a favorite
    Add {
        #[command(flatten)]
        args: crate::favorite::AddFavorite,
    },
    /// Delete a favorite by name
    Del { name: String },
    /// Edit a favorite
    Edit {
        #[command(flatten)]
        args: crate::favorite::EditFavorite,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        let (client, mut config) = crate::config::read(cli_config).await?;
        match self {
            Commands::Get => {
                crate::config::emit_value(&config.agents().get_favorites(), handle).await;
                Ok(())
            },
            Commands::Add { args } => {
                config.agents().add_favorite(args.into_favorite()?);
                crate::config::write(&client, &config, cli_config).await?;
                {
                objectiveai_sdk::cli::output::Output::<objectiveai_sdk::cli::output::Ok>::Notification(objectiveai_sdk::cli::output::Notification { value: objectiveai_sdk::cli::output::OK }).emit(handle).await;
                Ok(())
            }
            }
            Commands::Del { name } => {
                config.agents().del_favorite(&name)?;
                crate::config::write(&client, &config, cli_config).await?;
                {
                objectiveai_sdk::cli::output::Output::<objectiveai_sdk::cli::output::Ok>::Notification(objectiveai_sdk::cli::output::Notification { value: objectiveai_sdk::cli::output::OK }).emit(handle).await;
                Ok(())
            }
            }
            Commands::Edit { args } => {
                let favorite = config.agents().edit_favorite(&args.name)?;
                args.apply(favorite)?;
                crate::config::write(&client, &config, cli_config).await?;
                {
                objectiveai_sdk::cli::output::Output::<objectiveai_sdk::cli::output::Ok>::Notification(objectiveai_sdk::cli::output::Notification { value: objectiveai_sdk::cli::output::OK }).emit(handle).await;
                Ok(())
            }
            }
        }
    }
}
