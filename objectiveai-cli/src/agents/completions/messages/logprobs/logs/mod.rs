use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Get message logprobs, optionally filtered with jq
    Get {
        id: String,
        message_index: u64,
        filter: Option<String>,
    },
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
    pub async fn handle(
        self,
        cli_config: &crate::Config,
        handle: &objectiveai_sdk::cli::output::Handle,
    ) -> Result<(), crate::error::Error> {
        let client = crate::filesystem::Client::new(
            cli_config.config_base_dir.as_deref(),
            None::<String>,
            None::<String>,
        );
        match self {
            Commands::Get {
                id,
                message_index,
                filter,
            } => {
                let content = client
                    .read_agent_completion_message_logprobs(&id, message_index, filter.as_deref())
                    .await
                    .map(crate::filesystem::logs::LogContent::json)?;
                {
                    crate::log_line::emit_log_content(content, handle).await;
                    Ok(())
                }
            }
            Commands::Subscribe {
                id,
                message_index,
                timeout_ms,
                require_modification,
                filter,
            } => {
                let result = client
                    .subscribe_agent_completion_message_logprobs(
                        &id,
                        message_index,
                        std::time::Duration::from_millis(timeout_ms),
                        require_modification,
                        filter.as_deref(),
                    )
                    .await?;
                {
                    match result.map(crate::filesystem::logs::LogContent::json) {
                        Some(content) => {
                            crate::log_line::emit_log_content(content, handle).await;
                            Ok(())
                        }
                        None => Err(crate::error::Error::LogSubscribeTimedOut),
                    }
                }
            }
            Commands::Clear => {
                crate::log_line::emit_log_clear_count(
                    client.clear_agent_completion_message_logprobs().await?,
                    handle,
                )
                .await;
                Ok(())
            }
        }
    }
}
