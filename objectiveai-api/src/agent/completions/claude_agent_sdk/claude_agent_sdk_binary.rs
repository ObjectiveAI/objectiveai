/// Pre-built Claude Agent SDK runner binary (Python).
#[cfg(feature = "claude-agent-sdk")]
pub const CLAUDE_AGENT_SDK_RUNNER: &[u8] =
    include_bytes!(env!("OBJECTIVEAI_CLAUDE_AGENT_SDK_RUNNER_PATH"));
