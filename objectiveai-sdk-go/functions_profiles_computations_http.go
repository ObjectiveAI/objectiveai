package objectiveai

import "context"

func FunctionsProfilesComputationsComputeProfileUnary(ctx context.Context, c *Client, params FunctionsProfilesComputationsRequestFunctionProfileComputationCreateParams) (*FunctionsProfilesComputationsResponseUnaryFunctionProfileComputation, error) {
	params.Stream = nil
	return PostUnary[FunctionsProfilesComputationsResponseUnaryFunctionProfileComputation](ctx, c, "functions/profiles/compute", params)
}

func FunctionsProfilesComputationsComputeProfileStreaming(ctx context.Context, c *Client, params FunctionsProfilesComputationsRequestFunctionProfileComputationCreateParams) (*Stream[FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk], error) {
	params.Stream = ptrBool(true)
	return PostStreaming[FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk](ctx, c, "functions/profiles/compute", params)
}
