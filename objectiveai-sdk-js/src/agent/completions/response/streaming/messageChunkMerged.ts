import type { AgentCompletionsResponseStreamingMessageChunk } from "./messageChunk";
import { agentCompletionsResponseStreamingAssistantResponseChunkMerged } from "./assistantResponseChunkMerged";

function messageChunkIndex(chunk: AgentCompletionsResponseStreamingMessageChunk): number {
  return (chunk as { index: number }).index;
}

export function agentCompletionsResponseStreamingMessageChunkMerged(
  a: AgentCompletionsResponseStreamingMessageChunk,
  b: AgentCompletionsResponseStreamingMessageChunk,
): [AgentCompletionsResponseStreamingMessageChunk, boolean] {
  if (a.role === "assistant" && b.role === "assistant") {
    return agentCompletionsResponseStreamingAssistantResponseChunkMerged(a, b);
  }
  return [a, false];
}

export function agentCompletionsResponseStreamingMessageChunkMergedList(
  a: AgentCompletionsResponseStreamingMessageChunk[],
  b: AgentCompletionsResponseStreamingMessageChunk[],
): [AgentCompletionsResponseStreamingMessageChunk[], boolean] {
  let changed = false;
  const result = [...a];
  for (const bItem of b) {
    const bIndex = messageChunkIndex(bItem);
    const existingIdx = result.findIndex((x) => messageChunkIndex(x) === bIndex);
    if (existingIdx !== -1) {
      const [merged, c] = agentCompletionsResponseStreamingMessageChunkMerged(result[existingIdx], bItem);
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
