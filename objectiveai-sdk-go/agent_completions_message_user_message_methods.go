package objectiveai

// Push accumulates another UserMessage into this one.
func (v *AgentCompletionsMessageUserMessage) Push(other *AgentCompletionsMessageUserMessage) {
	v.Content.Push(&other.Content)
}
