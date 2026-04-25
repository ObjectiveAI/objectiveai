use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Drop every per-scope instructions table, wiping all stored IDs.
    /// Forces every streaming `create` command to require a fresh
    /// `instructions get` round-trip on its next invocation.
    Clear,
}

impl Commands {
    pub fn handle(self, cli_config: &crate::Config) -> Result<crate::Output, crate::error::Error> {
        match self {
            Commands::Clear => {
                let count = super::clear_all(cli_config)?;
                Ok(crate::Output::Instructions(format!(
                    "cleared {count} instruction tables"
                )))
            }
        }
    }
}
