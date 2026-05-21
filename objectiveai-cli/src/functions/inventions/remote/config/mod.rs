use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get the remote
    Get,
    /// Set the remote
    Set {
        #[arg(value_enum)]
        value: super::Remote,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        let (client, mut config) = crate::config::read(cli_config).await?;
        match self {
            Commands::Get => {
                crate::config::emit_value(&config.functions().inventions().get_remote(), handle).await;
                Ok(())
            },
            Commands::Set { value } => {
                config.functions().inventions().set_remote(value.into())?;
                crate::config::write(&client, &config, cli_config).await?;
                {
                objectiveai_sdk::cli::output::Output::<objectiveai_sdk::cli::output::Ok>::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: objectiveai_sdk::cli::output::OK }).emit(handle).await;
                Ok(())
            }
            }
        }
    }
}
