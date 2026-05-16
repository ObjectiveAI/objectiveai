package objectiveai

// Push accumulates another Logprobs into this one.
func (v *AgentCompletionsResponseLogprobs) Push(other *AgentCompletionsResponseLogprobs) {
	if v.Content != nil && other.Content != nil {
		*v.Content = append(*v.Content, *other.Content...)
	} else if other.Content != nil {
		cp := make([]AgentCompletionsResponseLogprob, len(*other.Content))
		copy(cp, *other.Content)
		v.Content = &cp
	}

	if v.Refusal != nil && other.Refusal != nil {
		*v.Refusal = append(*v.Refusal, *other.Refusal...)
	} else if other.Refusal != nil {
		cp := make([]AgentCompletionsResponseLogprob, len(*other.Refusal))
		copy(cp, *other.Refusal)
		v.Refusal = &cp
	}
}
