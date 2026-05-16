package objectiveai

import "context"

func VectorCompletionsCreateVectorCompletionUnary(ctx context.Context, c *Client, params VectorCompletionsRequestVectorCompletionCreateParams) (*VectorCompletionsResponseUnaryVectorCompletion, error) {
	params.Stream = nil
	return PostUnary[VectorCompletionsResponseUnaryVectorCompletion](ctx, c, "vector/completions", params)
}

func VectorCompletionsCreateVectorCompletionStreaming(ctx context.Context, c *Client, params VectorCompletionsRequestVectorCompletionCreateParams) (*Stream[VectorCompletionsResponseStreamingVectorCompletionChunk], error) {
	params.Stream = ptrBool(true)
	return PostStreaming[VectorCompletionsResponseStreamingVectorCompletionChunk](ctx, c, "vector/completions", params)
}
