use clap::{Args, Subcommand};
use futures::StreamExt;

crate::define_inline_or_ref!(AgentArg, "agent", objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional, Remote);

/// Shared params across all invention state types.
#[derive(Args)]
pub struct InventionParams {
    /// Function name
    #[arg(long)]
    pub name: String,
    /// Specification/prompt for the invention
    #[arg(long)]
    pub spec: String,
    /// Nesting depth (0 for leaf-only)
    #[arg(long, default_value = "0")]
    pub depth: u64,
    /// Minimum branch width
    #[arg(long, default_value = "2")]
    pub min_branch_width: u64,
    /// Maximum branch width
    #[arg(long, default_value = "3")]
    pub max_branch_width: u64,
    /// Minimum leaf width (tasks per leaf)
    #[arg(long, default_value = "1")]
    pub min_leaf_width: u64,
    /// Maximum leaf width (tasks per leaf)
    #[arg(long, default_value = "5")]
    pub max_leaf_width: u64,
}

impl InventionParams {
    fn into_params(self) -> objectiveai::functions::inventions::state::Params {
        objectiveai::functions::inventions::state::Params {
            depth: self.depth,
            min_branch_width: self.min_branch_width,
            max_branch_width: self.max_branch_width,
            min_leaf_width: self.min_leaf_width,
            max_leaf_width: self.max_leaf_width,
            name: self.name,
            spec: self.spec,
        }
    }
}

/// A single invention result item.
#[derive(serde::Serialize)]
pub struct InventionResultItem {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<objectiveai::RemotePath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<objectiveai::error::ResponseError>,
}

/// Extract the name from an invention State.
fn state_name(state: &objectiveai::functions::inventions::State) -> &str {
    match state {
        objectiveai::functions::inventions::State::AlphaScalarBranch(s) => &s.params.name,
        objectiveai::functions::inventions::State::AlphaScalarLeaf(s) => &s.params.name,
        objectiveai::functions::inventions::State::AlphaVectorBranch(s) => &s.params.name,
        objectiveai::functions::inventions::State::AlphaVectorLeaf(s) => &s.params.name,
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Invent a scalar function
    AlphaScalar {
        #[command(flatten)]
        params: InventionParams,
        #[command(flatten)]
        agent: AgentArg,
        #[command(flatten)]
        continuation: crate::continuation::ContinuationArgs,
        #[command(flatten)]
        instructions: crate::instructions::InstructionsIdArg,
        /// Seed for deterministic mock responses
        #[arg(long)]
        seed: Option<i64>,
        /// Run in the background: print PID and log path, then exit
        #[arg(long)]
        detach: bool,
    },
    /// Invent a vector function
    AlphaVector {
        #[command(flatten)]
        params: InventionParams,
        #[command(flatten)]
        agent: AgentArg,
        #[command(flatten)]
        continuation: crate::continuation::ContinuationArgs,
        #[command(flatten)]
        instructions: crate::instructions::InstructionsIdArg,
        /// Seed for deterministic mock responses
        #[arg(long)]
        seed: Option<i64>,
        /// Run in the background: print PID and log path, then exit
        #[arg(long)]
        detach: bool,
    },
    /// Invent from existing state (remote reference or inline JSON)
    Remote {
        /// State reference (e.g. remote=mock,name=inv-good-sl or remote=github,owner=x,repository=y)
        #[arg(long, required_unless_present = "state_inline")]
        state: Option<crate::path_ref::PathRef>,
        /// Inline JSON state
        #[arg(long, conflicts_with = "state")]
        state_inline: Option<String>,
        #[command(flatten)]
        agent: AgentArg,
        #[command(flatten)]
        continuation: crate::continuation::ContinuationArgs,
        #[command(flatten)]
        instructions: crate::instructions::InstructionsIdArg,
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
        let (agent_ref, continuation_args, instructions, seed, state, detach) = match self {
            Commands::AlphaScalar { params, agent, continuation, instructions, seed, detach } => {
                let p = params.into_params();
                let state = objectiveai::functions::inventions::ParamsStateOrRemoteCommitOptional::Inline(
                    objectiveai::functions::inventions::ParamsState::AlphaScalar(
                        objectiveai::functions::inventions::state::AlphaScalarState { params: p, input_schema: None },
                    ),
                );
                (agent, continuation, instructions, seed, state, detach)
            }
            Commands::AlphaVector { params, agent, continuation, instructions, seed, detach } => {
                let p = params.into_params();
                let state = objectiveai::functions::inventions::ParamsStateOrRemoteCommitOptional::Inline(
                    objectiveai::functions::inventions::ParamsState::AlphaVector(
                        objectiveai::functions::inventions::state::AlphaVectorState { params: p, input_schema: None },
                    ),
                );
                (agent, continuation, instructions, seed, state, detach)
            }
            Commands::Remote { state, state_inline, agent, continuation, instructions, seed, detach } => {
                let state = if let Some(inline) = state_inline {
                    let mut de = serde_json::Deserializer::from_str(&inline);
                    let parsed: objectiveai::functions::inventions::ParamsState =
                        serde_path_to_error::deserialize(&mut de)
                            .map_err(crate::error::Error::InlineDeserialize)?;
                    objectiveai::functions::inventions::ParamsStateOrRemoteCommitOptional::Inline(parsed)
                } else {
                    let remote_path = state.expect("clap ensures one is set").resolve()?;
                    objectiveai::functions::inventions::ParamsStateOrRemoteCommitOptional::Remote(remote_path)
                };
                (agent, continuation, instructions, seed, state, detach)
            }
        };

        instructions.verify(cli_config, crate::instructions::InstructionsScope::FunctionInventionsRecursive)?;

        if detach {
            crate::api::detach::detach().await;
        }

        let agent = agent_ref.resolve(|| async {
            let (_, mut c) = crate::config::read(cli_config).await.unwrap();
            c.agents().get_favorites().to_vec()
        }).await?;
        let continuation = continuation_args.resolve()?;

        // Read remote from config
        let (_, mut config) = crate::config::read(cli_config).await?;
        let remote = config.functions().inventions().get_remote();

        let request = objectiveai::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams {
            remote,
            overwrite: None,
            state,
            provider: None,
            agent,
            prompt: objectiveai::functions::inventions::prompts::InlinePromptOrRemoteCommitOptional::Remote(
                objectiveai::RemotePathCommitOptional::Mock { name: "default".to_string() },
            ),
            seed,
            stream: Some(true),
            max_step_retries: None,
            continuation,
        };

        let fs_client = objectiveai::filesystem::Client::new(cli_config.config_base_dir.as_deref(), None::<String>, None::<String>);
        let log_writer = objectiveai::filesystem::logs::client::write_function_invention_recursive(&fs_client);

        crate::api::run(Box::new(|http_client| Box::pin(async move {
            let stream = objectiveai::functions::inventions::recursive::create_function_invention_recursive_streaming(
                &http_client, request,
            ).await?;

            let chunk = crate::log_stream::consume_with_coalesced_writes(
                stream.map(|r| r.map_err(crate::error::Error::from)),
                log_writer,
                |agg: &mut objectiveai::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk, c| agg.push(c),
            ).await?;

            // Build result: one item per invention that has state
            let results: Vec<InventionResultItem> = chunk.inventions.iter()
                .filter_map(|inv| {
                    let state = inv.inner.state.as_ref()?;
                    Some(InventionResultItem {
                        name: state_name(state).to_string(),
                        path: inv.inner.path.clone(),
                        error: inv.inner.error.clone(),
                    })
                })
                .collect();

            Ok(serde_json::to_string(&results).unwrap())
        })), true).await
    }
}
