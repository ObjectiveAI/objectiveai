package objectiveai

import "context"

func SwarmListSwarms(ctx context.Context, c *Client, params SwarmListSwarmsRequest) (*SwarmListSwarmResponse, error) {
	return PostUnary[SwarmListSwarmResponse](ctx, c, "swarms/list", params)
}

func SwarmGetSwarm(ctx context.Context, c *Client, params RemotePathCommitOptional) (*SwarmGetSwarmResponse, error) {
	return PostUnary[SwarmGetSwarmResponse](ctx, c, "swarms", params)
}

func SwarmGetSwarmUsage(ctx context.Context, c *Client, params RemotePathCommitOptional) (*SwarmUsageSwarmResponse, error) {
	return PostUnary[SwarmUsageSwarmResponse](ctx, c, "swarms/usage", params)
}
