package objectiveai

// Push accumulates another DeveloperMessage into this one.
func (v *AgentCompletionsMessageDeveloperMessage) Push(other *AgentCompletionsMessageDeveloperMessage) {
	v.Content.Push(&other.Content)
}
