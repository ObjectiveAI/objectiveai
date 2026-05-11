use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Print the function-execution instructions and a fresh ID to pass
    /// to `create` via `--instructions-id`.
    Get,
}

impl Commands {
    pub fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        match self {
            Commands::Get => {
                #[derive(serde::Serialize)]
                struct Instructions { instructions: String }
                let instructions = 
                crate::instructions::issue(
                    cli_config,
                    crate::instructions::InstructionsScope::FunctionExecutions,
                    include_str!("../../../../assets/functions/executions/instructions/get/INSTRUCTIONS.md"),
                )?;
                objectiveai_cli_lib::output::Output::<Instructions>::Notification(Instructions { instructions }).emit();
                Ok(())
            },
        }
    }
}
