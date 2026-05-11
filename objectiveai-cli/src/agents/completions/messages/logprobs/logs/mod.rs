use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get message logprobs, optionally filtered with jq
    Get { id: String, message_index: u64, filter: Option<String> },
    /// Subscribe to changes (wait for create/modify), optionally filtered with jq
    Subscribe {
        id: String,
        message_index: u64,
        #[arg(long)]
        require_modification: bool,
        timeout_ms: u64,
        filter: Option<String>,
    },
    /// Clear all message logprobs
    Clear,
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        let client = objectiveai::filesystem::Client::new(cli_config.config_base_dir.as_deref(), None::<String>, None::<String>);
        match self {
            Commands::Get { id, message_index, filter } => {
                let content = objectiveai::filesystem::logs::client::read_agent_completion_message_logprobs(&client, &id, message_index, filter.as_deref()).await.map(objectiveai::filesystem::logs::LogContent::Json)?;
                {
                crate::ack::emit_log_content(content);
                Ok(())
            }
            }
            Commands::Subscribe { id, message_index, timeout_ms, require_modification, filter } => {
                let result = objectiveai::filesystem::logs::client::subscribe_agent_completion_message_logprobs(&client, &id, message_index, std::time::Duration::from_millis(timeout_ms), require_modification, filter.as_deref()).await?;
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
            Commands::Clear => {
                crate::ack::emit_log_clear_count(objectiveai::filesystem::logs::client::clear_agent_completion_message_logprobs(&client).await?);
                Ok(())
            },
        }
    }
}
