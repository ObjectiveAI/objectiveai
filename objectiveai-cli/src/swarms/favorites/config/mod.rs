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
                crate::config::emit_value(&config.swarms().get_favorites(), handle).await;
                Ok(())
            },
            Commands::Add { args } => {
                config.swarms().add_favorite(args.into_favorite()?);
                crate::config::write(&client, &config, cli_config).await?;
                {
                objectiveai_sdk::cli::output::Output::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: (objectiveai_sdk::cli::output::OK).into() }).emit(handle).await;
                Ok(())
            }
            }
            Commands::Del { name } => {
                config.swarms().del_favorite(&name)?;
                crate::config::write(&client, &config, cli_config).await?;
                {
                objectiveai_sdk::cli::output::Output::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: (objectiveai_sdk::cli::output::OK).into() }).emit(handle).await;
                Ok(())
            }
            }
            Commands::Edit { args } => {
                let favorite = config.swarms().edit_favorite(&args.name)?;
                args.apply(favorite)?;
                crate::config::write(&client, &config, cli_config).await?;
                {
                objectiveai_sdk::cli::output::Output::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: (objectiveai_sdk::cli::output::OK).into() }).emit(handle).await;
                Ok(())
            }
            }
        }
    }
}
