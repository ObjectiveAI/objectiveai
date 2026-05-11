use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get the value
    Get,
    /// Set the value
    Set { value: bool },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        let (client, mut config) = crate::config::read(cli_config).await?;
        match self {
            Commands::Get => {
                crate::config::emit_value(&config.api().local().get_claude_agent_sdk());
                Ok(())
            },
            Commands::Set { value } => {
                config.api().local().set_claude_agent_sdk(value);
                crate::config::write(&client, &config, cli_config).await?;
                {
                objectiveai_cli_lib::output::Output::<crate::ack::Ok>::Notification(crate::ack::OK).emit();
                Ok(())
            }
            }
        }
    }
}
