import type { LaboratoriesExecutionsResponseStreamingEvaluationChunk } from "./evaluationChunk";
import { agentCompletionsResponseStreamingMessageChunkMergedList } from "../../../../agent/completions/response/streaming/messageChunkMerged";
import { agentCompletionsResponseUsageMerged } from "../../../../agent/completions/response/usageMerged";

export function laboratoriesExecutionsResponseStreamingEvaluationChunkMerged(
  a: LaboratoriesExecutionsResponseStreamingEvaluationChunk,
  b: LaboratoriesExecutionsResponseStreamingEvaluationChunk,
): [LaboratoriesExecutionsResponseStreamingEvaluationChunk, boolean] {
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

  let output = a.output;
  if (b.output != null) {
    if (b.output !== a.output) changed = true;
    output = b.output;
  }

  // messages_queued: replace (latest non-null wins)
  let messages_queued = a.messages_queued;
  if (b.messages_queued != null) {
    if (b.messages_queued !== a.messages_queued) changed = true;
    messages_queued = b.messages_queued;
  }

  if (!changed) return [a, false];
  return [{
    index: a.index,
    agent_index: a.agent_index,
    id: a.id,
    created: a.created,
    messages,
    object: a.object,
    upstream: a.upstream,
    ...(usage != null ? { usage } : {}),
    ...(error != null ? { error } : {}),
    ...(continuation != null ? { continuation } : {}),
    ...(output != null ? { output } : {}),
    ...(messages_queued != null ? { messages_queued } : {}),
  }, true];
}

export function laboratoriesExecutionsResponseStreamingEvaluationChunkMergedList(
  a: LaboratoriesExecutionsResponseStreamingEvaluationChunk[],
  b: LaboratoriesExecutionsResponseStreamingEvaluationChunk[],
): [LaboratoriesExecutionsResponseStreamingEvaluationChunk[], boolean] {
  let changed = false;
  const result = [...a];
  for (const bItem of b) {
    const existingIdx = result.findIndex((x) => x.index === bItem.index);
    if (existingIdx !== -1) {
      const [merged, c] = laboratoriesExecutionsResponseStreamingEvaluationChunkMerged(result[existingIdx], bItem);
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
