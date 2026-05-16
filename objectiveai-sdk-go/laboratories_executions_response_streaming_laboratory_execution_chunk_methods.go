package objectiveai

// Push accumulates another LaboratoryExecutionChunk into this one.
func (v *LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk) Push(other *LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk) {
	// builders: merge by index
	pushByIndex(&v.Builders, other.Builders,
		func(t *LaboratoriesExecutionsResponseStreamingBuilderChunk) uint64 { return t.Index },
		func(a, b *LaboratoriesExecutionsResponseStreamingBuilderChunk) { a.Push(b) },
	)

	// evaluations: merge by index
	pushByIndex(&v.Evaluations, other.Evaluations,
		func(t *LaboratoriesExecutionsResponseStreamingEvaluationChunk) uint64 { return t.Index },
		func(a, b *LaboratoriesExecutionsResponseStreamingEvaluationChunk) { a.Push(b) },
	)

	// error: replace
	v.Error = pushReplace(v.Error, other.Error)

	// usage: delegate
	if v.Usage != nil && other.Usage != nil {
		v.Usage.Push(other.Usage)
	} else if other.Usage != nil {
		v.Usage = other.Usage
	}

	// id, created, object are immutable
}
