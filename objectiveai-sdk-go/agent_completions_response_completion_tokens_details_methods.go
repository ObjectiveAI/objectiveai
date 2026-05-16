package objectiveai

// Push accumulates another CompletionTokensDetails into this one.
func (v *AgentCompletionsResponseCompletionTokensDetails) Push(other *AgentCompletionsResponseCompletionTokensDetails) {
	v.AcceptedPredictionTokens = pushOptionUint64(v.AcceptedPredictionTokens, other.AcceptedPredictionTokens)
	v.AudioTokens = pushOptionUint64(v.AudioTokens, other.AudioTokens)
	v.ReasoningTokens = pushOptionUint64(v.ReasoningTokens, other.ReasoningTokens)
	v.RejectedPredictionTokens = pushOptionUint64(v.RejectedPredictionTokens, other.RejectedPredictionTokens)
}
