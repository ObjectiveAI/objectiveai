package objectiveai

import "context"

func FunctionsProfilesListProfiles(ctx context.Context, c *Client, params FunctionsProfilesListProfilesRequest) (*FunctionsProfilesListProfileResponse, error) {
	return PostUnary[FunctionsProfilesListProfileResponse](ctx, c, "functions/profiles/list", params)
}

func FunctionsProfilesGetProfile(ctx context.Context, c *Client, params RemotePathCommitOptional) (*FunctionsProfilesGetProfileResponse, error) {
	return PostUnary[FunctionsProfilesGetProfileResponse](ctx, c, "functions/profiles", params)
}

func FunctionsProfilesGetProfileUsage(ctx context.Context, c *Client, params RemotePathCommitOptional) (*FunctionsProfilesUsageProfileResponse, error) {
	return PostUnary[FunctionsProfilesUsageProfileResponse](ctx, c, "functions/profiles/usage", params)
}
