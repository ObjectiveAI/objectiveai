package objectiveai

// Push accumulates another inventions AgentCompletionChunk into this one.
func (v *FunctionsInventionsResponseStreamingAgentCompletionChunk) Push(other *FunctionsInventionsResponseStreamingAgentCompletionChunk) {
	// messages: merge by index
	pushByNullableIndex(&v.Messages, other.Messages,
		func(m *AgentCompletionsResponseStreamingMessageChunk) *uint64 { return m.Index() },
		func(a, b *AgentCompletionsResponseStreamingMessageChunk) { a.Push(b) },
	)

	// error: replace
	v.Error = pushReplace(v.Error, other.Error)

	// usage: delegate
	if v.Usage != nil && other.Usage != nil {
		v.Usage.Push(other.Usage)
	} else if other.Usage != nil {
		v.Usage = other.Usage
	}

	// continuation: replace
	v.Continuation = pushReplace(v.Continuation, other.Continuation)

	// messages_queued: replace (latest Some() wins)
	v.MessagesQueued = pushReplace(v.MessagesQueued, other.MessagesQueued)

	// id, created, object, upstream, index are immutable
}
