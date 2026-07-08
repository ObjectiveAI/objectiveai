package objectiveai

// Push accumulates another vector completions AgentCompletionChunk into this one.
func (v *VectorCompletionsResponseStreamingAgentCompletionChunk) Push(other *VectorCompletionsResponseStreamingAgentCompletionChunk) {
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

	// agent_inline: first chunk wins (rides only the completion's
	// first chunk; never overwritten)
	if v.AgentInline == nil {
		v.AgentInline = other.AgentInline
	}

	// request_choice_keys: first chunk wins (ride only the
	// completion's first chunk; never overwritten)
	if v.RequestChoiceKeys == nil {
		v.RequestChoiceKeys = other.RequestChoiceKeys
	}

	// id, created, object, upstream, index are immutable
}
