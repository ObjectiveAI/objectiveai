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
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        let (client, mut config) = crate::config::read(cli_config).await?;
        match self {
            Commands::Get => {
                crate::config::emit_value(&config.functions().inventions().get_remote());
                Ok(())
            },
            Commands::Set { value } => {
                config.functions().inventions().set_remote(value.into())?;
                crate::config::write(&client, &config, cli_config).await?;
                {
                objectiveai_cli_lib::output::Output::<crate::ack::Ok>::Notification(crate::ack::OK).emit();
                Ok(())
            }
            }
        }
    }
}
