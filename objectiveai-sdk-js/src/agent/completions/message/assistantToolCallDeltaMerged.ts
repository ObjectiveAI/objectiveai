import { merge } from "../../../merge";
import type { AgentCompletionsMessageAssistantToolCallDelta } from "./assistantToolCallDelta";
import { agentCompletionsMessageAssistantToolCallFunctionDeltaMerged } from "./assistantToolCallFunctionDeltaMerged";

export function agentCompletionsMessageAssistantToolCallDeltaMerged(
  a: AgentCompletionsMessageAssistantToolCallDelta,
  b: AgentCompletionsMessageAssistantToolCallDelta,
): [AgentCompletionsMessageAssistantToolCallDelta, boolean] {
  let changed = false;

  let type = a.type;
  if (a.type == null && b.type != null) {
    type = b.type;
    changed = true;
  }

  let id = a.id;
  if (a.id == null && b.id != null) {
    id = b.id;
    changed = true;
  }

  const [fn, fnChanged] = merge(
    a.function ?? undefined,
    b.function ?? undefined,
    agentCompletionsMessageAssistantToolCallFunctionDeltaMerged,
  );
  if (fnChanged) changed = true;

  if (!changed) return [a, false];
  return [{
    index: a.index,
    ...(type != null ? { type } : {}),
    ...(id != null ? { id } : {}),
    ...(fn != null ? { function: fn } : {}),
  }, true];
}

export function agentCompletionsMessageAssistantToolCallDeltaMergedList(
  a: AgentCompletionsMessageAssistantToolCallDelta[],
  b: AgentCompletionsMessageAssistantToolCallDelta[],
): [AgentCompletionsMessageAssistantToolCallDelta[], boolean] {
  let changed = false;
  const result = [...a];
  for (const bItem of b) {
    const existingIdx = result.findIndex((x) => x.index === bItem.index);
    if (existingIdx !== -1) {
      const [merged, c] = agentCompletionsMessageAssistantToolCallDeltaMerged(result[existingIdx], bItem);
      if (c) {
        result[existingIdx] = merged;
        changed = true;
      }
    } else {
      result.push(bItem);
      changed = true;
    }
  }
  if (!changed) return [a, false];
  return [result, true];
}
