package objectiveai

// Push accumulates another AssistantToolCallFunctionDelta into this one.
func (v *AgentCompletionsMessageAssistantToolCallFunctionDelta) Push(other *AgentCompletionsMessageAssistantToolCallFunctionDelta) {
	// name: lazy set (first wins)
	if v.Name == nil {
		v.Name = other.Name
	}
	// arguments: string concat
	v.Arguments = pushOptionString(v.Arguments, other.Arguments)
}
