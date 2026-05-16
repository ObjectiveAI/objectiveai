import type { AgentCompletionsResponseLogprobs } from "./logprobs";

export function agentCompletionsResponseLogprobsMerged(
  a: AgentCompletionsResponseLogprobs,
  b: AgentCompletionsResponseLogprobs,
): [AgentCompletionsResponseLogprobs, boolean] {
  let changed = false;

  let content = a.content;
  if (a.content != null && b.content != null) {
    if (b.content.length > 0) {
      content = [...a.content, ...b.content];
      changed = true;
    }
  } else if (b.content != null) {
    content = b.content;
    changed = true;
  }

  let refusal = a.refusal;
  if (a.refusal != null && b.refusal != null) {
    if (b.refusal.length > 0) {
      refusal = [...a.refusal, ...b.refusal];
      changed = true;
    }
  } else if (b.refusal != null) {
    refusal = b.refusal;
    changed = true;
  }

  if (!changed) return [a, false];
  return [{
    ...(content !== undefined ? { content } : {}),
    ...(refusal !== undefined ? { refusal } : {}),
  }, true];
}
