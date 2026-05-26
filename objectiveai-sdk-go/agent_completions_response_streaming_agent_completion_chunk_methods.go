package objectiveai

// Push accumulates another AgentCompletionChunk into this one.
func (v *AgentCompletionsResponseStreamingAgentCompletionChunk) Push(other *AgentCompletionsResponseStreamingAgentCompletionChunk) {
	// messages: merge by index
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

	// messages_queued: replace (latest Some() wins)
	v.MessagesQueued = pushReplace(v.MessagesQueued, other.MessagesQueued)

	// id, created, object, upstream are immutable
}
