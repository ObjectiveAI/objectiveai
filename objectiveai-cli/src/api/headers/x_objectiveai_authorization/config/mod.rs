use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Get,
    Set { value: String },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        let (client, mut config) = crate::config::read(cli_config).await?;
        match self {
            Commands::Get => {
                crate::config::emit_value(&config.api().headers().get_x_objectiveai_authorization());
                Ok(())
            },
            Commands::Set { value } => {
                config.api().headers().set_x_objectiveai_authorization(value);
                crate::config::write(&client, &config, cli_config).await?;
                {
                objectiveai_cli_lib::output::Output::<crate::ack::Ok>::Notification(crate::ack::OK).emit();
                Ok(())
            }
            }
        }
    }
}
