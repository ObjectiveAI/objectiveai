package objectiveai

// Push accumulates another VectorCompletionChunk into this one.
func (v *VectorCompletionsResponseStreamingVectorCompletionChunk) Push(other *VectorCompletionsResponseStreamingVectorCompletionChunk) {
	// completions: merge by index
	pushByIndex(&v.Completions, other.Completions,
		func(c *VectorCompletionsResponseStreamingAgentCompletionChunk) uint64 { return c.Index },
		func(a, b *VectorCompletionsResponseStreamingAgentCompletionChunk) { a.Push(b) },
	)

	// votes: extend
	v.Votes = append(v.Votes, other.Votes...)

	// scores: always replace
	v.Scores = make([]float64, len(other.Scores))
	copy(v.Scores, other.Scores)

	// weights: always replace
	v.Weights = make([]float64, len(other.Weights))
	copy(v.Weights, other.Weights)

	// usage: delegate
	if v.Usage != nil && other.Usage != nil {
		v.Usage.Push(other.Usage)
	} else if other.Usage != nil {
		v.Usage = other.Usage
	}

	// id, created, ensemble, object are immutable
}
