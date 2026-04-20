/// Pre-built Claude Agent SDK runner binary (JavaScript variant).
#[cfg(feature = "claude-agent-sdk-javascript")]
pub const CLAUDE_AGENT_SDK_RUNNER: &[u8] =
    include_bytes!(env!("OBJECTIVEAI_CLAUDE_AGENT_SDK_RUNNER_JS_PATH"));

/// Pre-built Claude Agent SDK runner binary (Python variant).
#[cfg(all(feature = "claude-agent-sdk-python", not(feature = "claude-agent-sdk-javascript")))]
pub const CLAUDE_AGENT_SDK_RUNNER: &[u8] =
    include_bytes!(env!("OBJECTIVEAI_CLAUDE_AGENT_SDK_RUNNER_PY_PATH"));
