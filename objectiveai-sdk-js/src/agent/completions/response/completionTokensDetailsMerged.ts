import { merge } from "../../../merge";
import type { AgentCompletionsResponseCompletionTokensDetails } from "./completionTokensDetails";

function mergedOptionU64(a: number | null | undefined, b: number | null | undefined): [number | null | undefined, boolean] {
  if (a != null && b != null) return [a + b, true];
  if (a != null) return [a, false];
  if (b != null) return [b, true];
  return [undefined, false];
}

export function agentCompletionsResponseCompletionTokensDetailsMerged(
  a: AgentCompletionsResponseCompletionTokensDetails,
  b: AgentCompletionsResponseCompletionTokensDetails,
): [AgentCompletionsResponseCompletionTokensDetails, boolean] {
  const [accepted_prediction_tokens, c1] = mergedOptionU64(a.accepted_prediction_tokens, b.accepted_prediction_tokens);
  const [audio_tokens, c2] = mergedOptionU64(a.audio_tokens, b.audio_tokens);
  const [reasoning_tokens, c3] = mergedOptionU64(a.reasoning_tokens, b.reasoning_tokens);
  const [rejected_prediction_tokens, c4] = mergedOptionU64(a.rejected_prediction_tokens, b.rejected_prediction_tokens);
  const changed = c1 || c2 || c3 || c4;
  if (!changed) return [a, false];
  return [{
    ...(accepted_prediction_tokens != null ? { accepted_prediction_tokens } : {}),
    ...(audio_tokens != null ? { audio_tokens } : {}),
    ...(reasoning_tokens != null ? { reasoning_tokens } : {}),
    ...(rejected_prediction_tokens != null ? { rejected_prediction_tokens } : {}),
  }, true];
}
