package objectiveai

// Push accumulates another AssistantResponseChunk into this one.
func (v *AgentCompletionsResponseStreamingAssistantResponseChunk) Push(other *AgentCompletionsResponseStreamingAssistantResponseChunk) {
	// reasoning: string concat
	v.Reasoning = pushOptionString(v.Reasoning, other.Reasoning)

	// tool_calls: merge by index
	if v.ToolCalls != nil && other.ToolCalls != nil {
		pushByIndex(v.ToolCalls, *other.ToolCalls,
			func(t *AgentCompletionsMessageAssistantToolCallDelta) uint64 { return t.Index },
			func(a, b *AgentCompletionsMessageAssistantToolCallDelta) { a.Push(b) },
		)
	} else if other.ToolCalls != nil {
		cp := make([]AgentCompletionsMessageAssistantToolCallDelta, len(*other.ToolCalls))
		copy(cp, *other.ToolCalls)
		v.ToolCalls = &cp
	}

	// content: delegate
	if v.Content != nil && other.Content != nil {
		v.Content.Push(other.Content)
	} else if other.Content != nil {
		v.Content = other.Content
	}

	// refusal: string concat
	v.Refusal = pushOptionString(v.Refusal, other.Refusal)

	// finish_reason: lazy set
	if v.FinishReason == nil {
		v.FinishReason = other.FinishReason
	}

	// logprobs: delegate
	if v.Logprobs != nil && other.Logprobs != nil {
		v.Logprobs.Push(other.Logprobs)
	} else if other.Logprobs != nil {
		v.Logprobs = other.Logprobs
	}

	// upstream_id: replace if empty -> non-empty
	if v.UpstreamID == "" && other.UpstreamID != "" {
		v.UpstreamID = other.UpstreamID
	}

	// service_tier: lazy set
	if v.ServiceTier == nil {
		v.ServiceTier = other.ServiceTier
	}

	// system_fingerprint: lazy set
	if v.SystemFingerprint == nil {
		v.SystemFingerprint = other.SystemFingerprint
	}

	// provider: lazy set
	if v.Provider == nil {
		v.Provider = other.Provider
	}

	// usage: delegate
	if v.Usage != nil && other.Usage != nil {
		v.Usage.Push(other.Usage)
	} else if other.Usage != nil {
		v.Usage = other.Usage
	}

	// role, index, created, agent, model are immutable
}
