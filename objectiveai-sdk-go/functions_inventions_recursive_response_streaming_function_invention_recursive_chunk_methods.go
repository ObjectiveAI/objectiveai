package objectiveai

// Push accumulates another FunctionInventionRecursiveChunk into this one.
func (v *FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk) Push(other *FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk) {
	// inventions: merge by index
	pushByIndex(&v.Inventions, other.Inventions,
		func(c *FunctionsInventionsRecursiveResponseStreamingFunctionInventionChunk) uint64 {
			return c.Index
		},
		func(a, b *FunctionsInventionsRecursiveResponseStreamingFunctionInventionChunk) { a.Push(b) },
	)

	// inventions_errors: lazy set true
	v.InventionsErrors = pushLazySetTrue(v.InventionsErrors, other.InventionsErrors)

	// usage: delegate
	if v.Usage != nil && other.Usage != nil {
		v.Usage.Push(other.Usage)
	} else if other.Usage != nil {
		v.Usage = other.Usage
	}

	// id, created, object are immutable
}
