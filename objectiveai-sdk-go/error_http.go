package objectiveai

import "context"

func ErrorCreateErrorUnary(ctx context.Context, c *Client, params ErrorErrorCreateParams) (*ErrorErrorResponse, error) {
	params.Stream = nil
	return PostUnary[ErrorErrorResponse](ctx, c, "error", params)
}

func ErrorCreateErrorStreaming(ctx context.Context, c *Client, params ErrorErrorCreateParams) (*Stream[ErrorErrorResponse], error) {
	params.Stream = ptrBool(true)
	return PostStreaming[ErrorErrorResponse](ctx, c, "error", params)
}
