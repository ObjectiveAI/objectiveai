use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get an agent completion log, optionally filtered with jq
    Get { id: String, filter: Option<String> },
    /// Subscribe to changes (wait for create/modify), optionally filtered with jq
    Subscribe {
        id: String,
        #[arg(long)]
        require_modification: bool,
        timeout_ms: u64,
        filter: Option<String>,
    },
    /// List agent completion logs
    List {
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Clear agent completion logs
    Clear {
        /// Also clear nested endpoints (continuations, messages, etc.)
        #[arg(long)]
        nested: bool,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        let client = objectiveai_sdk::filesystem::Client::new(cli_config.config_base_dir.as_deref(), None::<String>, None::<String>);
        match self {
            Commands::Get { id, filter } => {
                let content = client.read_agent_completion(&id, filter.as_deref()).await.map(objectiveai_sdk::filesystem::logs::LogContent::Json)?;
                {
                crate::log_line::emit_log_content(content, handle).await;
                Ok(())
            }
            }
            Commands::Subscribe { id, timeout_ms, require_modification, filter } => {
                let result = client.subscribe_agent_completion(&id, std::time::Duration::from_millis(timeout_ms), require_modification, filter.as_deref()).await?;
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
            Commands::List { offset, limit } => {
                {
                crate::log_line::emit_log_list(client.list_agent_completions(offset, limit).await?, handle).await;
                Ok(())
            }
            }
            Commands::Clear { nested } => {
                if nested {
                    let counts = futures::future::try_join_all(vec![
                        Box::pin(client.clear_agent_completions()) as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>,
                        Box::pin(client.clear_agent_completion_continuations()),
                        Box::pin(client.clear_agent_completion_messages()),
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
                crate::log_line::emit_log_clear_count(client.clear_agent_completions().await?, handle).await;
                Ok(())
            }
                }
            }
        }
    }
}
