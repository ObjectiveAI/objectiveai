import type { FunctionsInventionsResponseStreamingAgentCompletionChunk } from "./agentCompletionChunk";
import { agentCompletionsResponseStreamingMessageChunkMergedList } from "../../../../agent/completions/response/streaming/messageChunkMerged";
import { agentCompletionsResponseUsageMerged } from "../../../../agent/completions/response/usageMerged";

export function functionsInventionsResponseStreamingAgentCompletionChunkMerged(
  a: FunctionsInventionsResponseStreamingAgentCompletionChunk,
  b: FunctionsInventionsResponseStreamingAgentCompletionChunk,
): [FunctionsInventionsResponseStreamingAgentCompletionChunk, boolean] {
  let changed = false;

  const [messages, c1] = agentCompletionsResponseStreamingMessageChunkMergedList(a.messages, b.messages);
  if (c1) changed = true;

  let usage = a.usage;
  if (a.usage != null && b.usage != null) {
    const [merged, c] = agentCompletionsResponseUsageMerged(a.usage, b.usage);
    usage = merged;
    if (c) changed = true;
  } else if (b.usage != null) {
    usage = b.usage;
    changed = true;
  }

  let error = a.error;
  if (b.error != null) {
    if (b.error !== a.error) changed = true;
    error = b.error;
  }

  let continuation = a.continuation;
  if (b.continuation != null) {
    if (b.continuation !== a.continuation) changed = true;
    continuation = b.continuation;
  }

  if (!changed) return [a, false];
  return [{
    index: a.index,
    id: a.id,
    created: a.created,
    messages,
    object: a.object,
    ...(usage != null ? { usage } : {}),
    upstream: a.upstream,
    ...(error != null ? { error } : {}),
    ...(continuation != null ? { continuation } : {}),
  }, true];
}

export function functionsInventionsResponseStreamingAgentCompletionChunkMergedList(
  a: FunctionsInventionsResponseStreamingAgentCompletionChunk[],
  b: FunctionsInventionsResponseStreamingAgentCompletionChunk[],
): [FunctionsInventionsResponseStreamingAgentCompletionChunk[], boolean] {
  let changed = false;
  const result = [...a];
  for (const bItem of b) {
    const existingIdx = result.findIndex((x) => x.index === bItem.index);
    if (existingIdx !== -1) {
      const [merged, c] = functionsInventionsResponseStreamingAgentCompletionChunkMerged(result[existingIdx], bItem);
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
