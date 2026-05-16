package objectiveai

import "context"

func FunctionsExecutionsCreateFunctionExecutionUnary(ctx context.Context, c *Client, params FunctionsExecutionsRequestFunctionExecutionCreateParams) (*FunctionsExecutionsResponseUnaryFunctionExecution, error) {
	params.Stream = nil
	return PostUnary[FunctionsExecutionsResponseUnaryFunctionExecution](ctx, c, "functions/executions", params)
}

func FunctionsExecutionsCreateFunctionExecutionStreaming(ctx context.Context, c *Client, params FunctionsExecutionsRequestFunctionExecutionCreateParams) (*Stream[FunctionsExecutionsResponseStreamingFunctionExecutionChunk], error) {
	params.Stream = ptrBool(true)
	return PostStreaming[FunctionsExecutionsResponseStreamingFunctionExecutionChunk](ctx, c, "functions/executions", params)
}
