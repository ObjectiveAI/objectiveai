package objectiveai

// Push accumulates another recursive FunctionInventionChunk into this one.
func (v *FunctionsInventionsRecursiveResponseStreamingFunctionInventionChunk) Push(other *FunctionsInventionsRecursiveResponseStreamingFunctionInventionChunk) {
	// completions: merge by index
	pushByIndex(&v.Completions, other.Completions,
		func(c *FunctionsInventionsResponseStreamingAgentCompletionChunk) uint64 { return c.Index },
		func(a, b *FunctionsInventionsResponseStreamingAgentCompletionChunk) { a.Push(b) },
	)

	// state: replace
	v.State = pushReplace(v.State, other.State)

	// path: replace
	v.Path = pushReplace(v.Path, other.Path)

	// function: replace
	v.Function = pushReplace(v.Function, other.Function)

	// error: replace
	v.Error = pushReplace(v.Error, other.Error)

	// usage: delegate
	if v.Usage != nil && other.Usage != nil {
		v.Usage.Push(other.Usage)
	} else if other.Usage != nil {
		v.Usage = other.Usage
	}

	// id, created, object, index are immutable
}
