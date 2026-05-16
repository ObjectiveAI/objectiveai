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
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        let client = objectiveai_sdk::filesystem::Client::new(cli_config.config_base_dir.as_deref(), None::<String>, None::<String>);
        match self {
            Commands::Get { id, message_index, filter } => {
                let content = client.read_agent_completion_message(&id, message_index, filter.as_deref()).await.map(objectiveai_sdk::filesystem::logs::LogContent::Json)?;
                {
                crate::log_line::emit_log_content(content, handle).await;
                Ok(())
            }
            }
            Commands::Subscribe { id, message_index, timeout_ms, require_modification, filter } => {
                let result = client.subscribe_agent_completion_message(&id, message_index, std::time::Duration::from_millis(timeout_ms), require_modification, filter.as_deref()).await?;
                {
                match result.map(objectiveai_sdk::filesystem::logs::LogContent::Json) {
                    Some(content) => {
                        crate::log_line::emit_log_content(content, handle).await;
                        Ok(())
                    }
                    None => Err(crate::error::Error::LogSubscribeTimedOut),
                }
            }
            }
            Commands::Clear { nested } => {
                if nested {
                    let counts = futures::future::try_join_all(vec![
                        Box::pin(client.clear_agent_completion_messages()) as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>,
                        Box::pin(client.clear_agent_completion_message_logprobs()),
                        Box::pin(client.clear_agent_completion_message_images()),
                        Box::pin(client.clear_agent_completion_message_audio()),
                        Box::pin(client.clear_agent_completion_message_video()),
                        Box::pin(client.clear_agent_completion_message_files()),
                    ]).await?;
                    {
                crate::log_line::emit_log_clear_count(counts.into_iter().sum(), handle).await;
                Ok(())
            }
                } else {
                    {
                crate::log_line::emit_log_clear_count(client.clear_agent_completion_messages().await?, handle).await;
                Ok(())
            }
                }
            }
        }
    }
}
