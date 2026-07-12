package objectiveai

// Push accumulates another UpstreamDurationMs into this one — per-field
// sums across turns, fallbacks, and parallel agents (the Go mirror of
// the Rust `UpstreamDurationMs::push`). A field stays nil only when
// neither side recorded that upstream.
func (v *AgentCompletionsResponseUpstreamDurationMs) Push(other *AgentCompletionsResponseUpstreamDurationMs) {
	v.Openrouter = pushOptionUint64(v.Openrouter, other.Openrouter)
	v.ClaudeAgentSDK = pushOptionUint64(v.ClaudeAgentSDK, other.ClaudeAgentSDK)
	v.CodexSDK = pushOptionUint64(v.CodexSDK, other.CodexSDK)
}
