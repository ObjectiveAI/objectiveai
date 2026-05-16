package objectiveai

// Push accumulates another BuilderChunk into this one.
func (v *LaboratoriesExecutionsResponseStreamingBuilderChunk) Push(other *LaboratoriesExecutionsResponseStreamingBuilderChunk) {
	// messages: merge by nullable index
	pushByNullableIndex(&v.Messages, other.Messages,
		func(m *AgentCompletionsResponseStreamingMessageChunk) *uint64 { return m.Index() },
		func(a, b *AgentCompletionsResponseStreamingMessageChunk) { a.Push(b) },
	)

	// usage: delegate
	if v.Usage != nil && other.Usage != nil {
		v.Usage.Push(other.Usage)
	} else if other.Usage != nil {
		v.Usage = other.Usage
	}

	// error: replace
	v.Error = pushReplace(v.Error, other.Error)

	// continuation: replace
	v.Continuation = pushReplace(v.Continuation, other.Continuation)

	// index, agent_index, id, created, object, upstream are immutable
}
