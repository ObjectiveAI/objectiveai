use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get pairs configuration
    Get {
        #[arg(short, long)]
        filter: Option<String>,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        let (_, mut config) = crate::config::read(cli_config).await?;
        match self {
            Commands::Get { filter } => crate::config::emit_jq(config.functions().profiles().pairs().jq(&crate::config::filter(filter))),
        }
    }
}
