package objectiveai

import "context"

func FunctionsInventionsCreateFunctionInventionUnary(ctx context.Context, c *Client, params FunctionsInventionsRequestFunctionInventionCreateParams) (*FunctionsInventionsResponseUnaryFunctionInvention, error) {
	params.Stream = nil
	return PostUnary[FunctionsInventionsResponseUnaryFunctionInvention](ctx, c, "functions/inventions", params)
}

func FunctionsInventionsCreateFunctionInventionStreaming(ctx context.Context, c *Client, params FunctionsInventionsRequestFunctionInventionCreateParams) (*Stream[FunctionsInventionsResponseStreamingFunctionInventionChunk], error) {
	params.Stream = ptrBool(true)
	return PostStreaming[FunctionsInventionsResponseStreamingFunctionInventionChunk](ctx, c, "functions/inventions", params)
}
