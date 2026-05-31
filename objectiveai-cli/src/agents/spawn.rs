//! `agents spawn` — fire a child agent in the background. Always
//! detaches; emits exactly one `Spawned { agent_id }` notification
//! (the spawned agent's local lineage segment) and exits. The
//! actual completion stream is consumed by an orphaned
//! `objectiveai-cli-stream` child, which writes coalesced log
//! files under `${config_base_dir}/logs/`.

use clap::Args;

use objectiveai_sdk::agent::completions::message::{Message, RichContent, UserMessage};

crate::define_inline_or_ref!(AgentArg, "agent", objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional, Remote);

/// How the prompt is provided to the agent completion. Resolves to
/// the wire-level `messages` array, but the CLI surface is named
/// "prompt" — the common case (`--simple "<text>"`) is a single
/// user message, not a multi-message conversation.
#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct PromptSource {
    /// Plain text — becomes one user message
    /// (`{ role: "user", content: <text> }`).
    #[arg(long)]
    simple: Option<String>,
    /// Inline JSON messages array
    #[arg(long)]
    inline: Option<String>,
    /// Path to a JSON file containing the messages array
    #[arg(long)]
    file: Option<std::path::PathBuf>,
    /// Inline Python code that produces the messages array
    #[arg(long)]
    python_inline: Option<String>,
    /// Path to a Python file that produces the messages array
    #[arg(long)]
    python_file: Option<std::path::PathBuf>,
}

impl PromptSource {
    fn resolve(self) -> Result<Vec<Message>, crate::error::Error> {
        crate::source_resolver::resolve_source(
            self.simple,
            self.inline,
            self.file,
            self.python_inline,
            self.python_file,
            |text| {
                vec![Message::User(UserMessage {
                    content: RichContent::Text(text),
                    name: None,
                })]
            },
        )
    }
}

#[derive(Args)]
pub struct CommandArgs {
    #[command(flatten)]
    pub prompt: PromptSource,
    #[command(flatten)]
    pub agent: AgentArg,
    /// Seed for deterministic mock responses
    #[arg(long)]
    pub seed: Option<i64>,
}

pub async fn handle(
    args: CommandArgs,
    cli_config: &crate::Config,
    handle: &objectiveai_sdk::cli::output::Handle,
) -> Result<(), crate::error::Error> {
    let messages = args.prompt.resolve()?;
    let agent = args
        .agent
        .resolve(|| async {
            let (_, mut c) = crate::config::read(cli_config).await.unwrap();
            c.agents().get_favorites().to_vec()
        })
        .await?;

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages,
        provider: None,
        agent,
        response_format: None,
        seed: args.seed,
        stream: Some(true),
        continuation: None,
    };

    crate::api::stream_subprocess::run_detached(
        cli_config,
        &["agents", "spawn"],
        &params,
        handle,
    )
    .await
}
