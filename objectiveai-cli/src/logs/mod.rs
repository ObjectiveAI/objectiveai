use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Clear all logs across all endpoints
    Clear,
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        let client = objectiveai::filesystem::Client::new(cli_config.config_base_dir.as_deref(), None::<String>, None::<String>);
        match self {
            Commands::Clear => {
                let counts = futures::future::try_join_all(vec![
                    Box::pin(objectiveai::filesystem::logs::client::clear_agent_completions(&client)) as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>,
                    Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_continuations(&client)),
                    Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_messages(&client)),
                    Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_logprobs(&client)),
                    Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_images(&client)),
                    Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_audio(&client)),
                    Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_video(&client)),
                    Box::pin(objectiveai::filesystem::logs::client::clear_agent_completion_message_files(&client)),
                    Box::pin(objectiveai::filesystem::logs::client::clear_vector_completions(&client)),
                    Box::pin(objectiveai::filesystem::logs::client::clear_function_executions(&client)),
                    Box::pin(objectiveai::filesystem::logs::client::clear_function_execution_retry_tokens(&client)),
                    Box::pin(objectiveai::filesystem::logs::client::clear_function_inventions(&client)),
                    Box::pin(objectiveai::filesystem::logs::client::clear_function_inventions_recursive(&client)),
                    Box::pin(objectiveai::filesystem::logs::client::clear_laboratory_executions(&client)),
                ]).await?;
                {
                crate::ack::emit_log_clear_count(counts.into_iter().sum());
                Ok(())
            }
            }
        }
    }
}
