use clap::{Args, Subcommand};
use futures::StreamExt;

crate::define_inline_or_ref!(AgentArg, "agent", objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional, Remote);

/// How messages are provided to the agent completion.
#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct MessageSource {
    /// Inline JSON messages array
    #[arg(long)]
    messages_inline: Option<String>,
    /// Inline Python code that produces the messages array
    #[arg(long)]
    messages_python_inline: Option<String>,
    /// Path to a Python file that produces the messages array
    #[arg(long)]
    messages_python_file: Option<std::path::PathBuf>,
}

impl MessageSource {
    fn resolve(self) -> Result<Vec<objectiveai::agent::completions::message::Message>, crate::error::Error> {
        if let Some(inline) = self.messages_inline {
            let mut de = serde_json::Deserializer::from_str(&inline);
            return serde_path_to_error::deserialize(&mut de)
                .map_err(crate::error::Error::InlineDeserialize);
        }
        if let Some(code) = self.messages_python_inline {
            return crate::python::exec_code(&code);
        }
        if let Some(path) = self.messages_python_file {
            return crate::python::exec_file(&path);
        }
        unreachable!("clap group ensures one is set")
    }
}


#[derive(Subcommand)]
pub enum Commands {
    /// Standard agent completion
    Standard {
        #[command(flatten)]
        messages: MessageSource,
        #[command(flatten)]
        agent: AgentArg,
        #[command(flatten)]
        continuation: crate::continuation::ContinuationArgs,
        #[command(flatten)]
        response_format: crate::response_format::ResponseFormatArgs,
        /// Seed for deterministic mock responses
        #[arg(long)]
        seed: Option<i64>,
        /// Run in the background: print PID and log path, then exit
        #[arg(long)]
        detach: bool,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<crate::Output, crate::error::Error> {
        let (message_source, agent_arg, continuation_args, response_format_args, seed, detach) = match self {
            Commands::Standard { messages, agent, continuation, response_format, seed, detach } => {
                (messages, agent, continuation, response_format, seed, detach)
            }
        };

        if detach {
            crate::api::detach::detach().await;
        }

        let messages = message_source.resolve()?;
        let agent = agent_arg.resolve(|| async {
            let (_, mut c) = crate::config::read(cli_config).await.unwrap();
            c.agents().get_favorites().to_vec()
        }).await?;
        let continuation = continuation_args.resolve()?;
        let response_format = response_format_args.resolve()?
            .map(objectiveai::agent::completions::request::ResponseFormatParam::Single);

        let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
            messages,
            provider: None,
            agent,
            response_format,
            seed,
            stream: Some(true),
            continuation,
        };

        let fs_client = objectiveai::filesystem::Client::new(cli_config.config_base_dir.as_deref(), None::<String>, None::<String>);
        let log_writer = objectiveai::filesystem::logs::client::write_agent_completion(&fs_client);

        crate::api::run(Box::new(|http_client| Box::pin(async move {
            let stream = objectiveai::agent::completions::create_agent_completion_streaming(
                &http_client, params,
            ).await?;

            let accumulated = crate::log_stream::consume_with_coalesced_writes(
                stream.map(|r| r.map_err(crate::error::Error::from)),
                log_writer,
                |agg: &mut objectiveai::agent::completions::response::streaming::AgentCompletionChunk, c| agg.push(c),
            ).await?;

            let completion: objectiveai::agent::completions::response::unary::AgentCompletion = accumulated.into();

            // Extract the last assistant message content
            let content = completion.messages.iter().rev()
                .find_map(|msg| {
                    if let objectiveai::agent::completions::response::unary::Message::Assistant(asst) = msg {
                        asst.content.as_ref().map(|c| match c {
                            objectiveai::agent::completions::message::RichContent::Text(t) => t.clone(),
                            objectiveai::agent::completions::message::RichContent::Parts(parts) => {
                                parts.iter().filter_map(|p| match p {
                                    objectiveai::agent::completions::message::RichContentPart::Text { text } => Some(text.as_str()),
                                    _ => None,
                                }).collect::<Vec<_>>().join("")
                            }
                        })
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            Ok(content)
        })), true).await
    }
}
