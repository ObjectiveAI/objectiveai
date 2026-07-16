package objectiveai

// Push accumulates another Usage into this one.
func (v *AgentCompletionsResponseUsage) Push(other *AgentCompletionsResponseUsage) {
	v.CompletionTokens += other.CompletionTokens
	v.PromptTokens += other.PromptTokens
	v.TotalTokens += other.TotalTokens
	v.Cost += other.Cost
	v.TotalCost += other.TotalCost

	if v.CompletionTokensDetails != nil && other.CompletionTokensDetails != nil {
		v.CompletionTokensDetails.Push(other.CompletionTokensDetails)
	} else if other.CompletionTokensDetails != nil {
		v.CompletionTokensDetails = other.CompletionTokensDetails
	}

	if v.PromptTokensDetails != nil && other.PromptTokensDetails != nil {
		v.PromptTokensDetails.Push(other.PromptTokensDetails)
	} else if other.PromptTokensDetails != nil {
		v.PromptTokensDetails = other.PromptTokensDetails
	}

	if v.CostDetails != nil && other.CostDetails != nil {
		v.CostDetails.Push(other.CostDetails)
	} else if other.CostDetails != nil {
		v.CostDetails = other.CostDetails
	}

	v.UpstreamDurationMs.Push(&other.UpstreamDurationMs)
}
