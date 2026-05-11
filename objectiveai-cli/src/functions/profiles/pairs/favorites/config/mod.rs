use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get all pair favorites
    Get,
    /// Add a pair favorite
    Add {
        #[command(flatten)]
        args: crate::favorite::AddPairFavorite,
    },
    /// Delete a pair favorite by name
    Del { name: String },
    /// Edit a pair favorite
    Edit {
        #[command(flatten)]
        args: crate::favorite::EditPairFavorite,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        let (client, mut config) = crate::config::read(cli_config).await?;
        match self {
            Commands::Get => {
                crate::config::emit_value(&config.functions().profiles().pairs().get_favorites());
                Ok(())
            }
            Commands::Add { args } => {
                config.functions().profiles().pairs().add_favorite(args.into_pair_favorite()?);
                crate::config::write(&client, &config, cli_config).await?;
                objectiveai_cli_lib::output::Output::<crate::ack::Ok>::Notification(crate::ack::OK).emit();
                Ok(())
            }
            Commands::Del { name } => {
                config.functions().profiles().pairs().del_favorite(&name)?;
                crate::config::write(&client, &config, cli_config).await?;
                objectiveai_cli_lib::output::Output::<crate::ack::Ok>::Notification(crate::ack::OK).emit();
                Ok(())
            }
            Commands::Edit { args } => {
                let favorite = config.functions().profiles().pairs().edit_favorite(&args.name)?;
                args.apply(favorite)?;
                crate::config::write(&client, &config, cli_config).await?;
                objectiveai_cli_lib::output::Output::<crate::ack::Ok>::Notification(crate::ack::OK).emit();
                Ok(())
            }
        }
    }
}
