package objectiveai

// Push accumulates another ReasoningSummaryChunk into this one.
func (v *FunctionsExecutionsResponseStreamingReasoningSummaryChunk) Push(other *FunctionsExecutionsResponseStreamingReasoningSummaryChunk) {
	// messages: merge by index
	pushByNullableIndex(&v.Messages, other.Messages,
		func(m *AgentCompletionsResponseStreamingMessageChunk) *uint64 { return m.Index() },
		func(a, b *AgentCompletionsResponseStreamingMessageChunk) { a.Push(b) },
	)

	// error: replace
	v.Error = pushReplace(v.Error, other.Error)

	// continuation: replace
	v.Continuation = pushReplace(v.Continuation, other.Continuation)

	// usage: delegate
	if v.Usage != nil && other.Usage != nil {
		v.Usage.Push(other.Usage)
	} else if other.Usage != nil {
		v.Usage = other.Usage
	}

	// messages_queued: replace (latest Some() wins)
	v.MessagesQueued = pushReplace(v.MessagesQueued, other.MessagesQueued)

	// agent_inline: first chunk wins (rides only the completion's
	// first chunk; never overwritten)
	if v.AgentInline == nil {
		v.AgentInline = other.AgentInline
	}

	// id, created, object, upstream are immutable
}
