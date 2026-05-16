import type { AgentCompletionsResponsePromptTokensDetails } from "./promptTokensDetails";

function mergedOptionU64(a: number | null | undefined, b: number | null | undefined): [number | null | undefined, boolean] {
  if (a != null && b != null) return [a + b, true];
  if (a != null) return [a, false];
  if (b != null) return [b, true];
  return [undefined, false];
}

export function agentCompletionsResponsePromptTokensDetailsMerged(
  a: AgentCompletionsResponsePromptTokensDetails,
  b: AgentCompletionsResponsePromptTokensDetails,
): [AgentCompletionsResponsePromptTokensDetails, boolean] {
  const [audio_tokens, c1] = mergedOptionU64(a.audio_tokens, b.audio_tokens);
  const [cached_tokens, c2] = mergedOptionU64(a.cached_tokens, b.cached_tokens);
  const [cache_write_tokens, c3] = mergedOptionU64(a.cache_write_tokens, b.cache_write_tokens);
  const [video_tokens, c4] = mergedOptionU64(a.video_tokens, b.video_tokens);
  const changed = c1 || c2 || c3 || c4;
  if (!changed) return [a, false];
  return [{
    ...(audio_tokens != null ? { audio_tokens } : {}),
    ...(cached_tokens != null ? { cached_tokens } : {}),
    ...(cache_write_tokens != null ? { cache_write_tokens } : {}),
    ...(video_tokens != null ? { video_tokens } : {}),
  }, true];
}
