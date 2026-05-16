package objectiveai

import "context"

func FunctionsListFunctions(ctx context.Context, c *Client, params FunctionsListFunctionsRequest) (*FunctionsListFunctionResponse, error) {
	return PostUnary[FunctionsListFunctionResponse](ctx, c, "functions/list", params)
}

func FunctionsGetFunction(ctx context.Context, c *Client, params RemotePathCommitOptional) (*FunctionsGetFunctionResponse, error) {
	return PostUnary[FunctionsGetFunctionResponse](ctx, c, "functions", params)
}

func FunctionsGetFunctionUsage(ctx context.Context, c *Client, params RemotePathCommitOptional) (*FunctionsUsageFunctionResponse, error) {
	return PostUnary[FunctionsUsageFunctionResponse](ctx, c, "functions/usage", params)
}

func FunctionsListFunctionProfilePairs(ctx context.Context, c *Client, params FunctionsListFunctionProfilePairsRequest) (*FunctionsListFunctionProfilePairResponse, error) {
	return PostUnary[FunctionsListFunctionProfilePairResponse](ctx, c, "functions/profiles/pairs/list", params)
}

func FunctionsGetFunctionProfilePairUsage(ctx context.Context, c *Client, params FunctionsGetFunctionProfilePairUsageRequest) (*FunctionsUsageFunctionProfilePairResponse, error) {
	return PostUnary[FunctionsUsageFunctionProfilePairResponse](ctx, c, "functions/profiles/pairs/usage", params)
}
