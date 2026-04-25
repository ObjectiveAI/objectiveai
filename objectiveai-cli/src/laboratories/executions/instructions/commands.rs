use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Print the laboratory-execution instructions and a fresh ID to pass
    /// to `create` via `--instructions-id`.
    Get,
}

impl Commands {
    pub fn handle(self, cli_config: &crate::Config) -> Result<crate::Output, crate::error::Error> {
        match self {
            Commands::Get => Ok(crate::Output::Instructions(
                crate::instructions::issue(
                    cli_config,
                    crate::instructions::InstructionsScope::LaboratoryExecutions,
                    include_str!("../../../../assets/laboratories/executions/instructions/get/INSTRUCTIONS.md"),
                )?,
            )),
        }
    }
}
