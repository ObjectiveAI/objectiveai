use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get a message log, optionally filtered with jq
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
    /// Clear message logs
    Clear {
        /// Also clear nested endpoints (logprobs, image, audio, video, file)
        #[arg(long)]
        nested: bool,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<crate::Output, crate::error::Error> {
        let client = objectiveai::filesystem::Client::new(cli_config.config_base_dir.as_deref(), None::<String>, None::<String>);
        match self {
            Commands::Get { id, message_index, filter } => {
                let content = objectiveai::filesystem::logs::client::read_agent_completion_message(&client, &id, message_index, filter.as_deref()).await.map(objectiveai::filesystem::logs::LogContent::Json)?;
                Ok(crate::Output::LogsGet(content))
            }
            Commands::Subscribe { id, message_index, timeout_ms, require_modification, filter } => {
                let result = objectiveai::filesystem::logs::client::subscribe_agent_completion_message(&client, &id, message_index, std::time::Duration::from_millis(timeout_ms), require_modification, filter.as_deref()).await?;
                Ok(crate::Output::LogsSubscribe(result.map(objectiveai::filesystem::logs::LogContent::Json)))
            }
            Commands::Clear { nested } => {
                if nested {
                    let counts = futures::future::try_join_all(vec![
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_messages(&client)) as std::pin::Pin<Box<dyn std::future::Future<Output = _>>>,
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_logprobs(&client)),
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_images(&client)),
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_audio(&client)),
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_video(&client)),
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_files(&client)),
                    ]).await?;
                    Ok(crate::Output::LogsClear(counts.into_iter().sum()))
                } else {
                    Ok(crate::Output::LogsClear(objectiveai::filesystem::logs::client::clear_agent_completion_messages(&client).await?))
                }
            }
        }
    }
}
