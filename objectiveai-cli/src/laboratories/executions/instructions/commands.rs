use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Print the laboratory-execution instructions and a fresh ID to pass
    /// to `create` via `--instructions-id`.
    Get,
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Get => {
                #[derive(serde::Serialize)]
                struct Instructions { instructions: String }
                let instructions = 
                crate::instructions::issue(
                    cli_config,
                    crate::instructions::InstructionsScope::LaboratoryExecutions,
                    include_str!("../../../../assets/laboratories/executions/instructions/get/INSTRUCTIONS.md"),
                )?;
                objectiveai_sdk::cli::output::Output::<Instructions>::Notification(objectiveai_sdk::cli::output::Notification { value: Instructions { instructions } }).emit(handle).await;
                Ok(())
            },
        }
    }
}
