package objectiveai

// Index returns the index from whichever variant is set.
func (v *FunctionsExecutionsResponseStreamingTaskChunk) Index() *uint64 {
	if v.FunctionExecution != nil {
		return &v.FunctionExecution.Index
	}
	if v.VectorCompletion != nil {
		return &v.VectorCompletion.Index
	}
	return nil
}

// Push accumulates another TaskChunk into this one.
// Variant dispatch: only merges if both are the same variant type.
func (v *FunctionsExecutionsResponseStreamingTaskChunk) Push(other *FunctionsExecutionsResponseStreamingTaskChunk) {
	if v.FunctionExecution != nil && other.FunctionExecution != nil {
		v.FunctionExecution.Push(other.FunctionExecution)
	} else if v.VectorCompletion != nil && other.VectorCompletion != nil {
		v.VectorCompletion.Push(other.VectorCompletion)
	}
}
