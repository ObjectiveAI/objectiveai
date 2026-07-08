package objectiveai

// Push accumulates another VectorCompletionTaskChunk into this one.
func (v *FunctionsExecutionsResponseStreamingVectorCompletionTaskChunk) Push(other *FunctionsExecutionsResponseStreamingVectorCompletionTaskChunk) {
	// completions: merge by index
	pushByIndex(&v.Completions, other.Completions,
		func(c *VectorCompletionsResponseStreamingAgentCompletionChunk) uint64 { return c.Index },
		func(a, b *VectorCompletionsResponseStreamingAgentCompletionChunk) { a.Push(b) },
	)

	// votes: extend
	v.Votes = append(v.Votes, other.Votes...)

	// scores: replace (if non-empty)
	if len(other.Scores) > 0 {
		v.Scores = make([]float64, len(other.Scores))
		copy(v.Scores, other.Scores)
	}

	// weights: replace (if non-empty)
	if len(other.Weights) > 0 {
		v.Weights = make([]float64, len(other.Weights))
		copy(v.Weights, other.Weights)
	}

	// error: replace
	v.Error = pushReplace(v.Error, other.Error)

	// usage: delegate
	if v.Usage != nil && other.Usage != nil {
		v.Usage.Push(other.Usage)
	} else if other.Usage != nil {
		v.Usage = other.Usage
	}

	// request_messages / request_choices: first chunk wins (ride only
	// the task's first chunk; never overwritten)
	if v.RequestMessages == nil {
		v.RequestMessages = other.RequestMessages
	}
	if v.RequestChoices == nil {
		v.RequestChoices = other.RequestChoices
	}

	// id, created, object, ensemble, index, task_index, task_path are immutable
}
