package objectiveai

// Push accumulates another SystemMessage into this one.
func (v *AgentCompletionsMessageSystemMessage) Push(other *AgentCompletionsMessageSystemMessage) {
	v.Content.Push(&other.Content)
}
