use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Clear all logs across all endpoints
    Clear,
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        let client = objectiveai_sdk::filesystem::Client::new(cli_config.config_base_dir.as_deref(), None::<String>, None::<String>);
        match self {
            Commands::Clear => {
                let counts = futures::future::try_join_all(vec![
                    Box::pin(client.clear_agent_completions()) as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>,
                    Box::pin(client.clear_agent_completion_continuations()),
                    Box::pin(client.clear_agent_completion_messages()),
                    Box::pin(client.clear_agent_completion_message_logprobs()),
                    Box::pin(client.clear_agent_completion_message_images()),
                    Box::pin(client.clear_agent_completion_message_audio()),
                    Box::pin(client.clear_agent_completion_message_video()),
                    Box::pin(client.clear_agent_completion_message_files()),
                    Box::pin(client.clear_vector_completions()),
                    Box::pin(client.clear_function_executions()),
                    Box::pin(client.clear_function_execution_retry_tokens()),
                    Box::pin(client.clear_function_inventions()),
                    Box::pin(client.clear_function_inventions_recursive()),
                    Box::pin(client.clear_laboratory_executions()),
                ]).await?;
                {
                crate::log_line::emit_log_clear_count(counts.into_iter().sum(), handle).await;
                Ok(())
            }
            }
        }
    }
}
