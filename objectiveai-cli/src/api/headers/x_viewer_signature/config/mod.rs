use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Get,
    Set { value: String },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        let (client, mut config) = crate::config::read(cli_config).await?;
        match self {
            Commands::Get => {
                crate::config::emit_value(&config.api().headers().get_x_viewer_signature(), handle).await;
                Ok(())
            },
            Commands::Set { value } => {
                config.api().headers().set_x_viewer_signature(value);
                crate::config::write(&client, &config, cli_config).await?;
                {
                objectiveai_sdk::cli::output::Output::<objectiveai_sdk::cli::output::Ok>::Notification(objectiveai_sdk::cli::output::Notification { value: objectiveai_sdk::cli::output::OK }).emit(handle).await;
                Ok(())
            }
            }
        }
    }
}
