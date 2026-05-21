use clap::Args;

/// Optional `--agent-id` flag flattened into every api endpoint
/// leaf's `Args` struct. Sets the request's `X-OBJECTIVEAI-AGENT-ID`
/// header only when `Handle.agent_id` (env-derived) is `None`;
/// otherwise the env value wins.
#[derive(Args, Debug, Clone)]
pub struct AgentIdArg {
    /// Optional agent ID for the request's `X-OBJECTIVEAI-AGENT-ID`
    /// header. Ignored when `OBJECTIVEAI_AGENT_ID` is set in the
    /// environment — the env value (which also feeds
    /// `Handle.agent_id`) always wins.
    #[arg(long)]
    pub agent_id: Option<String>,
}
