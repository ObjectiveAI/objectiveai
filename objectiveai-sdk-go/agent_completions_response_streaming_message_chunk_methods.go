package objectiveai

// Index returns the index from whichever variant is set.
func (v *AgentCompletionsResponseStreamingMessageChunk) Index() *uint64 {
	if v.Assistant != nil {
		return &v.Assistant.Index
	}
	if v.Tool != nil {
		return &v.Tool.Index
	}
	return nil
}

// Push accumulates another MessageChunk into this one.
// Only merges if both are assistant variants.
func (v *AgentCompletionsResponseStreamingMessageChunk) Push(other *AgentCompletionsResponseStreamingMessageChunk) {
	if v.Assistant != nil && other.Assistant != nil {
		v.Assistant.Push(other.Assistant)
	}
}
