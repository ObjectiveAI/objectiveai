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
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        let client = objectiveai::filesystem::Client::new(cli_config.config_base_dir.as_deref(), None::<String>, None::<String>);
        match self {
            Commands::Get { id, filter } => {
                let content = objectiveai::filesystem::logs::client::read_agent_completion(&client, &id, filter.as_deref()).await.map(objectiveai::filesystem::logs::LogContent::Json)?;
                {
                crate::ack::emit_log_content(content);
                Ok(())
            }
            }
            Commands::Subscribe { id, timeout_ms, require_modification, filter } => {
                let result = objectiveai::filesystem::logs::client::subscribe_agent_completion(&client, &id, std::time::Duration::from_millis(timeout_ms), require_modification, filter.as_deref()).await?;
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
                {
                crate::ack::emit_log_list(objectiveai::filesystem::logs::client::list_agent_completions(&client, offset, limit).await?);
                Ok(())
            }
            }
            Commands::Clear { nested } => {
                if nested {
                    let counts = futures::future::try_join_all(vec![
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completions(&client)) as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>,
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_continuations(&client)),
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_messages(&client)),
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_logprobs(&client)),
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_images(&client)),
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_audio(&client)),
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_video(&client)),
                        Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_files(&client)),
                    ]).await?;
                    {
                crate::ack::emit_log_clear_count(counts.into_iter().sum());
                Ok(())
            }
                } else {
                    {
                crate::ack::emit_log_clear_count(objectiveai::filesystem::logs::client::clear_agent_completions(&client).await?);
                Ok(())
            }
                }
            }
        }
    }
}
