package objectiveai

import "context"

func FunctionsInventionsStateGetFunctionInventionState(ctx context.Context, c *Client, params RemotePathCommitOptional) (*FunctionsInventionsStateGetFunctionInventionStateResponse, error) {
	return PostUnary[FunctionsInventionsStateGetFunctionInventionStateResponse](ctx, c, "functions/inventions/state", params)
}
