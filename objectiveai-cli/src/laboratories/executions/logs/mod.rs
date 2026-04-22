use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get a laboratory execution log, optionally filtered with jq
    Get { id: String, filter: Option<String> },
    /// Subscribe to changes (wait for create/modify), optionally filtered with jq
    Subscribe {
        id: String,
        #[arg(long)]
        require_modification: bool,
        timeout_ms: u64,
        filter: Option<String>,
    },
    /// List laboratory execution logs
    List {
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Clear all laboratory execution logs
    Clear,
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<crate::Output, crate::error::Error> {
        let client = objectiveai::filesystem::Client::new(cli_config.config_base_dir.as_deref(), None::<String>, None::<String>);
        match self {
            Commands::Get { id, filter } => {
                let content = objectiveai::filesystem::logs::client::read_laboratory_execution(&client, &id, filter.as_deref()).await.map(objectiveai::filesystem::logs::LogContent::Json)?;
                Ok(crate::Output::LogsGet(content))
            }
            Commands::Subscribe { id, timeout_ms, require_modification, filter } => {
                let result = objectiveai::filesystem::logs::client::subscribe_laboratory_execution(&client, &id, std::time::Duration::from_millis(timeout_ms), require_modification, filter.as_deref()).await?;
                Ok(crate::Output::LogsSubscribe(result.map(objectiveai::filesystem::logs::LogContent::Json)))
            }
            Commands::List { offset, limit } => {
                Ok(crate::Output::LogsList(objectiveai::filesystem::logs::client::list_laboratory_executions(&client, offset, limit).await?))
            }
            Commands::Clear => Ok(crate::Output::LogsClear(objectiveai::filesystem::logs::client::clear_laboratory_executions(&client).await?)),
        }
    }
}
