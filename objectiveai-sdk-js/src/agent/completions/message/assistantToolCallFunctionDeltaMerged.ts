import { merge, mergedString } from "../../../merge";
import type { AgentCompletionsMessageAssistantToolCallFunctionDelta } from "./assistantToolCallFunctionDelta";

export function agentCompletionsMessageAssistantToolCallFunctionDeltaMerged(
  a: AgentCompletionsMessageAssistantToolCallFunctionDelta,
  b: AgentCompletionsMessageAssistantToolCallFunctionDelta,
): [AgentCompletionsMessageAssistantToolCallFunctionDelta, boolean] {
  let changed = false;

  let name = a.name;
  if (a.name == null && b.name != null) {
    name = b.name;
    changed = true;
  }

  let args = a.arguments;
  if (a.arguments != null && b.arguments != null) {
    const [merged, c] = mergedString(a.arguments, b.arguments);
    args = merged;
    if (c) changed = true;
  } else if (b.arguments != null) {
    args = b.arguments;
    changed = true;
  }

  if (!changed) return [a, false];
  return [{
    ...(name != null ? { name } : {}),
    ...(args != null ? { arguments: args } : {}),
  }, true];
}
