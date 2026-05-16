use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get all MCP entries
    Get,
    /// Add an MCP authorization entry
    Add { key: String, value: String },
    /// Remove an MCP authorization entry
    Del { key: String },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        let (client, mut config) = crate::config::read(cli_config).await?;
        match self {
            Commands::Get => {
                crate::config::emit_value(&config.api().headers().get_x_mcp_authorization(), handle).await;
                Ok(())
            },
            Commands::Add { key, value } => {
                config.api().headers().add_x_mcp_authorization(key, value);
                crate::config::write(&client, &config, cli_config).await?;
                {
                objectiveai_sdk::cli::output::Output::<objectiveai_sdk::cli::output::Ok>::Notification(objectiveai_sdk::cli::output::Notification { value: objectiveai_sdk::cli::output::OK }).emit(handle).await;
                Ok(())
            }
            }
            Commands::Del { key } => {
                config.api().headers().del_x_mcp_authorization(&key);
                crate::config::write(&client, &config, cli_config).await?;
                {
                objectiveai_sdk::cli::output::Output::<objectiveai_sdk::cli::output::Ok>::Notification(objectiveai_sdk::cli::output::Notification { value: objectiveai_sdk::cli::output::OK }).emit(handle).await;
                Ok(())
            }
            }
        }
    }
}
