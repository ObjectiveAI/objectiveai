import OpenAI from "openai";
import { CacheVoteRequest } from "./request";
import { CacheVote } from "./response";

export async function retrieve(
  openai: OpenAI,
  request: CacheVoteRequest,
  options?: OpenAI.RequestOptions,
): Promise<CacheVote> {
  const response = await openai.get("/vector/completions/cache", {
    query: request as unknown as Record<string, unknown>,
    ...options,
  });
  return response as CacheVote;
}
