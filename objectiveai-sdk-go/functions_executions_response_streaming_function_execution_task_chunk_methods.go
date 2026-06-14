package objectiveai

// Push accumulates another FunctionExecutionTaskChunk into this one.
func (v *FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk) Push(other *FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk) {
	// tasks: merge by index
	pushByNullableIndex(&v.Tasks, other.Tasks,
		func(t *FunctionsExecutionsResponseStreamingTaskChunk) *uint64 { return t.Index() },
		func(a, b *FunctionsExecutionsResponseStreamingTaskChunk) { a.Push(b) },
	)

	// tasks_errors: lazy set true
	v.TasksErrors = pushLazySetTrue(v.TasksErrors, other.TasksErrors)

	// reasoning: delegate
	if v.Reasoning != nil && other.Reasoning != nil {
		v.Reasoning.Push(other.Reasoning)
	} else if other.Reasoning != nil {
		v.Reasoning = other.Reasoning
	}

	// output: replace
	v.Output = pushReplace(v.Output, other.Output)

	// error: replace
	v.Error = pushReplace(v.Error, other.Error)

	// usage: delegate
	if v.Usage != nil && other.Usage != nil {
		v.Usage.Push(other.Usage)
	} else if other.Usage != nil {
		v.Usage = other.Usage
	}

	// id, created, object, function, profile, index, task_index, task_path,
	// swiss_round, swiss_pool_index are immutable
}
