package objectiveai

import "context"

func VectorCompletionsCacheGetCompletionVotes(ctx context.Context, c *Client, params VectorCompletionsCacheGetCompletionVotesRequest) (*VectorCompletionsCacheCompletionVotes, error) {
	return PostUnary[VectorCompletionsCacheCompletionVotes](ctx, c, "vector/completions/votes", params)
}

func VectorCompletionsCacheGetCacheVote(ctx context.Context, c *Client, params VectorCompletionsCacheCacheVoteRequest) (*VectorCompletionsCacheCacheVote, error) {
	return PostUnary[VectorCompletionsCacheCacheVote](ctx, c, "vector/completions/cache", params)
}
