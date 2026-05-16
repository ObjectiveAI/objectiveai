package objectiveai

import "context"

func AgentListAgents(ctx context.Context, c *Client, params AgentListAgentsRequest) (*AgentListAgentResponse, error) {
	return PostUnary[AgentListAgentResponse](ctx, c, "agents/list", params)
}

func AgentGetAgent(ctx context.Context, c *Client, params RemotePathCommitOptional) (*AgentGetAgentResponse, error) {
	return PostUnary[AgentGetAgentResponse](ctx, c, "agents", params)
}

func AgentGetAgentUsage(ctx context.Context, c *Client, params RemotePathCommitOptional) (*AgentUsageAgentResponse, error) {
	return PostUnary[AgentUsageAgentResponse](ctx, c, "agents/usage", params)
}
