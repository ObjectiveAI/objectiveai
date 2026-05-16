package objectiveai

import "context"

func AgentCompletionsCreateAgentCompletionUnary(ctx context.Context, c *Client, params AgentCompletionsRequestAgentCompletionCreateParams) (*AgentCompletionsResponseUnaryAgentCompletion, error) {
	params.Stream = nil
	return PostUnary[AgentCompletionsResponseUnaryAgentCompletion](ctx, c, "agent/completions", params)
}

func AgentCompletionsCreateAgentCompletionStreaming(ctx context.Context, c *Client, params AgentCompletionsRequestAgentCompletionCreateParams) (*Stream[AgentCompletionsResponseStreamingAgentCompletionChunk], error) {
	params.Stream = ptrBool(true)
	return PostStreaming[AgentCompletionsResponseStreamingAgentCompletionChunk](ctx, c, "agent/completions", params)
}

func AgentCompletionsNotifyAgentCompletion(ctx context.Context, c *Client, params AgentCompletionsRequestAgentCompletionNotifyParams) error {
	return PostNoResponse(ctx, c, "agent/completions/notify", params)
}

func ptrBool(v bool) *bool { return &v }
