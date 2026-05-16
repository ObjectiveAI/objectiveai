package objectiveai

// Push accumulates another AssistantToolCallDelta into this one.
func (v *AgentCompletionsMessageAssistantToolCallDelta) Push(other *AgentCompletionsMessageAssistantToolCallDelta) {
	// type: lazy set
	if v.Type == nil {
		v.Type = other.Type
	}
	// id: lazy set
	if v.ID == nil {
		v.ID = other.ID
	}
	// function: delegate
	if v.Function != nil && other.Function != nil {
		v.Function.Push(other.Function)
	} else if other.Function != nil {
		v.Function = other.Function
	}
}
