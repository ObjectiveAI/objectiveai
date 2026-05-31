use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get the value
    Get,
    /// Set the value
    Set { value: u16 },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        let (client, mut config) = crate::config::read(cli_config).await?;
        match self {
            Commands::Get => {
                crate::config::emit_value(&config.mcp().get_port(), handle).await;
                Ok(())
            }
            Commands::Set { value } => {
                config.mcp().set_port(value);
                crate::config::write(&client, &config, cli_config).await?;
                objectiveai_sdk::cli::output::Output::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: (objectiveai_sdk::cli::output::OK).into() }).emit(handle).await;
                Ok(())
            }
        }
    }
}
