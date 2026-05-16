package objectiveai

import "context"

func AuthCreateApiKey(ctx context.Context, c *Client, params AuthCreateApiKeyRequest) (*AuthApiKeyWithMetadata, error) {
	return PostUnary[AuthApiKeyWithMetadata](ctx, c, "auth/keys", params)
}

func AuthCreateOpenrouterByokApiKey(ctx context.Context, c *Client, params AuthCreateOpenRouterByokApiKeyRequest) (*AuthGetOpenRouterByokApiKeyResponse, error) {
	return PostUnary[AuthGetOpenRouterByokApiKeyResponse](ctx, c, "auth/keys/openrouter", params)
}

func AuthDisableApiKey(ctx context.Context, c *Client, params AuthDisableApiKeyRequest) (*AuthApiKeyWithMetadata, error) {
	return DeleteUnary[AuthApiKeyWithMetadata](ctx, c, "auth/keys", params)
}

func AuthDeleteOpenrouterByokApiKey(ctx context.Context, c *Client) error {
	return DeleteNoContent(ctx, c, "auth/keys/openrouter", nil)
}

func AuthListApiKeys(ctx context.Context, c *Client) (*AuthListApiKeyResponse, error) {
	return GetUnary[AuthListApiKeyResponse](ctx, c, "auth/keys", nil)
}

func AuthGetOpenrouterByokApiKey(ctx context.Context, c *Client) (*AuthGetOpenRouterByokApiKeyResponse, error) {
	return GetUnary[AuthGetOpenRouterByokApiKeyResponse](ctx, c, "auth/keys/openrouter", nil)
}

func AuthGetCredits(ctx context.Context, c *Client) (*AuthGetCreditsResponse, error) {
	return GetUnary[AuthGetCreditsResponse](ctx, c, "auth/credits", nil)
}
