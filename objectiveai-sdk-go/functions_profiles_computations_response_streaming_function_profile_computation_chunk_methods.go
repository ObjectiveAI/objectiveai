package objectiveai

// Push accumulates another FunctionProfileComputationChunk into this one.
func (v *FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk) Push(other *FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk) {
	// executions: merge by index
	pushByIndex(&v.Executions, other.Executions,
		func(e *FunctionsProfilesComputationsResponseStreamingFunctionExecutionChunk) uint64 {
			return e.Index
		},
		func(a, b *FunctionsProfilesComputationsResponseStreamingFunctionExecutionChunk) { a.Push(b) },
	)

	// executions_errors: lazy set true
	v.ExecutionsErrors = pushLazySetTrue(v.ExecutionsErrors, other.ExecutionsErrors)

	// profile: replace
	v.Profile = pushReplace(v.Profile, other.Profile)

	// fitting_stats: replace
	v.FittingStats = pushReplace(v.FittingStats, other.FittingStats)

	// usage: delegate
	if v.Usage != nil && other.Usage != nil {
		v.Usage.Push(other.Usage)
	} else if other.Usage != nil {
		v.Usage = other.Usage
	}

	// id, created, object, function are immutable
}
