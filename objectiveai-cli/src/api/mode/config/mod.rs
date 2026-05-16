use clap::{Subcommand, ValueEnum};

#[derive(Clone, ValueEnum)]
pub enum Mode {
    Remote,
    Local,
}

impl From<Mode> for objectiveai_sdk::filesystem::config::ApiMode {
    fn from(m: Mode) -> Self {
        match m {
            Mode::Remote => objectiveai_sdk::filesystem::config::ApiMode::Remote,
            Mode::Local => objectiveai_sdk::filesystem::config::ApiMode::Local,
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Get the mode
    Get,
    /// Set the mode
    Set {
        #[arg(value_enum)]
        value: Mode,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        let (client, mut config) = crate::config::read(cli_config).await?;
        match self {
            Commands::Get => {
                crate::config::emit_value(config.api().get_mode(), handle).await;
                Ok(())
            }
            Commands::Set { value } => {
                config.api().set_mode(value.into());
                crate::config::write(&client, &config, cli_config).await?;
                objectiveai_sdk::cli::output::Output::<objectiveai_sdk::cli::output::Ok>::Notification(objectiveai_sdk::cli::output::Notification { value: objectiveai_sdk::cli::output::OK }).emit(handle).await;
                Ok(())
            }
        }
    }
}
