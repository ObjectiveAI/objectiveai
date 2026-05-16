package objectiveai

import "context"

func FunctionsInventionsRecursiveCreateFunctionInventionRecursiveUnary(ctx context.Context, c *Client, params FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParams) (*FunctionsInventionsRecursiveResponseUnaryFunctionInventionRecursive, error) {
	params.Stream = nil
	return PostUnary[FunctionsInventionsRecursiveResponseUnaryFunctionInventionRecursive](ctx, c, "functions/inventions/recursive", params)
}

func FunctionsInventionsRecursiveCreateFunctionInventionRecursiveStreaming(ctx context.Context, c *Client, params FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParams) (*Stream[FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk], error) {
	params.Stream = ptrBool(true)
	return PostStreaming[FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk](ctx, c, "functions/inventions/recursive", params)
}
