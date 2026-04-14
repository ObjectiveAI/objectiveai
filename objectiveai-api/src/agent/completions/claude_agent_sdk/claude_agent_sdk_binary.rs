/// Pre-built `objectiveai-claude-agent-sdk-runner` binary.
/// Same target as the API server build target.
pub const CLAUDE_AGENT_SDK_RUNNER: &[u8] =
    include_bytes!(env!("OBJECTIVEAI_CLAUDE_AGENT_SDK_RUNNER_PATH"));
