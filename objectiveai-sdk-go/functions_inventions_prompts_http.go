package objectiveai

import "context"

func FunctionsInventionsPromptsListPrompts(ctx context.Context, c *Client, params FunctionsInventionsPromptsListPromptsRequest) (*FunctionsInventionsPromptsListPromptResponse, error) {
	return PostUnary[FunctionsInventionsPromptsListPromptResponse](ctx, c, "functions/inventions/prompts/list", params)
}

func FunctionsInventionsPromptsGetPrompt(ctx context.Context, c *Client, params RemotePathCommitOptional) (*FunctionsInventionsPromptsGetPromptResponse, error) {
	return PostUnary[FunctionsInventionsPromptsGetPromptResponse](ctx, c, "functions/inventions/prompts", params)
}

func FunctionsInventionsPromptsGetPromptUsage(ctx context.Context, c *Client, params RemotePathCommitOptional) (*FunctionsInventionsPromptsUsagePromptResponse, error) {
	return PostUnary[FunctionsInventionsPromptsUsagePromptResponse](ctx, c, "functions/inventions/prompts/usage", params)
}
