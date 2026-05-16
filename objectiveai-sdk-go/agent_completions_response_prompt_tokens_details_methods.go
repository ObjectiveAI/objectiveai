package objectiveai

// Push accumulates another PromptTokensDetails into this one.
func (v *AgentCompletionsResponsePromptTokensDetails) Push(other *AgentCompletionsResponsePromptTokensDetails) {
	v.AudioTokens = pushOptionUint64(v.AudioTokens, other.AudioTokens)
	v.CachedTokens = pushOptionUint64(v.CachedTokens, other.CachedTokens)
	v.CacheWriteTokens = pushOptionUint64(v.CacheWriteTokens, other.CacheWriteTokens)
	v.VideoTokens = pushOptionUint64(v.VideoTokens, other.VideoTokens)
}
