use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Print the agent-completion instructions and a fresh ID to pass
    /// to `create` via `--instructions-id`.
    Get,
}

impl Commands {
    pub fn handle(self, cli_config: &crate::Config) -> Result<crate::Output, crate::error::Error> {
        match self {
            Commands::Get => Ok(crate::Output::Instructions(
                crate::instructions::issue(
                    cli_config,
                    crate::instructions::InstructionsScope::AgentCompletions,
                    include_str!("../../../../assets/agents/completions/instructions/get/INSTRUCTIONS.md"),
                )?,
            )),
        }
    }
}
