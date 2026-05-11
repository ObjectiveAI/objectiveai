use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get a function invention log, optionally filtered with jq
    Get { id: String, filter: Option<String> },
    /// Subscribe to changes (wait for create/modify), optionally filtered with jq
    Subscribe {
        id: String,
        #[arg(long)]
        require_modification: bool,
        timeout_ms: u64,
        filter: Option<String>,
    },
    /// List function invention logs
    List {
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Clear all function invention logs
    Clear,
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        let client = objectiveai::filesystem::Client::new(cli_config.config_base_dir.as_deref(), None::<String>, None::<String>);
        match self {
            Commands::Get { id, filter } => {
                let content = objectiveai::filesystem::logs::client::read_function_invention(&client, &id, filter.as_deref()).await.map(objectiveai::filesystem::logs::LogContent::Json)?;
                {
                crate::ack::emit_log_content(content);
                Ok(())
            }
            }
            Commands::Subscribe { id, timeout_ms, require_modification, filter } => {
                let result = objectiveai::filesystem::logs::client::subscribe_function_invention(&client, &id, std::time::Duration::from_millis(timeout_ms), require_modification, filter.as_deref()).await?;
                {
                match result.map(objectiveai::filesystem::logs::LogContent::Json) {
                    Some(content) => {
                        crate::ack::emit_log_content(content);
                        Ok(())
                    }
                    None => Err(crate::error::Error::LogSubscribeTimedOut),
                }
            }
            }
            Commands::List { offset, limit } => {
                crate::ack::emit_log_list(objectiveai::filesystem::logs::client::list_function_inventions(&client, offset, limit).await?);
                Ok(())
            },
            Commands::Clear => {
                crate::ack::emit_log_clear_count(objectiveai::filesystem::logs::client::clear_function_inventions(&client).await?);
                Ok(())
            },
        }
    }
}
